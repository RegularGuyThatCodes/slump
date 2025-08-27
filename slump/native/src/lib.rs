mod audio;
mod error;
mod video;
mod webrtc;

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use audio::AudioCapture;
use error::Result;
use napi::{
    bindgen_prelude::*,
    threadsafe_function::{ThreadSafeCallContext, ThreadsafeFunction, ThreadsafeFunctionCallMode},
    JsFunction,
};
use napi_derive::napi;
use video::VideoCapture;
use webrtc::{SignalMessage, WebRTCTransport};

struct SlumpStream {
    video_capture: Option<VideoCapture>,
    audio_capture: Option<AudioCapture>,
    transport: Option<WebRTCTransport>,
    running: bool,
    stats: Arc<Mutex<StreamStats>>,
}

#[derive(Default, Clone)]
struct StreamStats {
    video_frames_sent: u64,
    audio_frames_sent: u64,
    video_bitrate: f64,
    audio_bitrate: f64,
    rtt: f64,
    jitter: f64,
    timestamp: Instant,
}

impl Default for SlumpStream {
    fn default() -> Self {
        Self {
            video_capture: None,
            audio_capture: None,
            transport: None,
            running: false,
            stats: Arc::new(Mutex::new(StreamStats::default())),
        }
    }
}

static mut STREAM: Option<SlumpStream> = None;
static STREAM_INIT: std::sync::Once = std::sync::Once::new();

#[napi]
pub fn start_stream(
    width: u32,
    height: u32,
    fps: u32,
    bitrate: u32,
    stun_servers: Vec<String>,
    on_event: JsFunction,
) -> napi::Result<bool> {
    STREAM_INIT.call_once(|| unsafe {
        STREAM = Some(SlumpStream::default());
    });

    let stream = unsafe { STREAM.as_mut() }.ok_or_else(|| {
        napi::Error::new(
            napi::Status::GenericFailure,
            "Failed to initialize stream".to_string(),
        )
    })?;

    if stream.running {
        return Ok(false);
    }

    // Initialize video capture
    stream.video_capture = Some(
        VideoCapture::new(0, width, height).map_err(|e| {
            napi::Error::new(
                napi::Status::GenericFailure,
                format!("Failed to initialize video capture: {}", e),
            )
        })?,
    );

    // Initialize audio capture
    stream.audio_capture = Some(AudioCapture::new().map_err(|e| {
        napi::Error::new(
            napi::Status::GenericFailure,
            format!("Failed to initialize audio capture: {}", e),
        )
    })?);

    // Initialize WebRTC transport
    let transport = tokio::runtime::Runtime::new()
        .map_err(|e| {
            napi::Error::new(
                napi::Status::GenericFailure,
                format!("Failed to create runtime: {}", e),
            )
        })?
        .block_on(async {
            WebRTCTransport::new(stun_servers, vec![]).await.map_err(|e| {
                napi::Error::new(
                    napi::Status::GenericFailure,
                    format!("Failed to create WebRTC transport: {}", e),
                )
            })
        })??;

    stream.transport = Some(transport);
    stream.running = true;

    // Start streaming loop in a separate thread
    let stats_clone = stream.stats.clone();
    let on_event_ts: ThreadsafeFunction<StreamEvent> = on_event
        .create_threadsafe_function(0, |ctx: ThreadSafeCallContext<StreamEvent>| {
            Ok(vec![ctx.value])
        })?;

    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let mut video_interval = tokio::time::interval(Duration::from_millis(1000 / fps as u64));
            let mut stats_interval = tokio::time::interval(Duration::from_secs(1));
            let mut last_stats_time = Instant::now();
            let mut last_video_bytes = 0;
            let mut last_audio_bytes = 0;

            loop {
                tokio::select! {
                    _ = video_interval.tick() => {
                        // Capture and send video frame
                        if let (Some(video), Some(transport)) = 
                            (unsafe { STREAM.as_mut() }.and_then(|s| s.video_capture.as_mut()), 
                             unsafe { STREAM.as_mut() }.and_then(|s| s.transport.as_mut())) 
                        {
                            if let Ok(Some(frame)) = video.capture_frame() {
                                if let Err(e) = transport.send_video_frame(&frame, 0).await {
                                    log::error!("Failed to send video frame: {}", e);
                                }
                                let mut stats = stats_clone.lock().unwrap();
                                stats.video_frames_sent += 1;
                                stats.video_bitrate = (frame.len() as f64 * 8.0 * fps as f64) / 1000.0;
                            }
                        }
                    }
                    _ = stats_interval.tick() => {
                        // Update and emit stats
                        let now = Instant::now();
                        let elapsed = now.duration_since(last_stats_time).as_secs_f64();
                        last_stats_time = now;

                        let stats = stats_clone.lock().unwrap();
                        let video_kbps = stats.video_bitrate;
                        let audio_kbps = stats.audio_bitrate;
                        let rtt = stats.rtt;
                        let jitter = stats.jitter;
                        let fps = stats.video_frames_sent as f64 / elapsed;
                        
                        let _ = on_event_ts.call_async(StreamEvent::Stats {
                            video_kbps,
                            audio_kbps,
                            rtt,
                            jitter,
                            fps,
                        });
                    }
                    else => break,
                }
            }
        });
    });

    Ok(true)
}

#[napi]
pub fn stop_stream() -> napi::Result<bool> {
    let stream = unsafe { STREAM.as_mut() }.ok_or_else(|| {
        napi::Error::new(
            napi::Status::GenericFailure,
            "Stream not initialized".to_string(),
        )
    })?;

    if !stream.running {
        return Ok(false);
    }

    stream.running = false;
    stream.video_capture = None;
    stream.audio_capture = None;
    stream.transport = None;

    Ok(true)
}

#[napi(object)]
pub struct Stats {
    pub video_kbps: f64,
    pub audio_kbps: f64,
    pub rtt: f64,
    pub jitter: f64,
    pub fps: f64,
}

#[napi]
pub fn get_stats() -> napi::Result<Stats> {
    let stream = unsafe { STREAM.as_ref() }.ok_or_else(|| {
        napi::Error::new(
            napi::Status::GenericFailure,
            "Stream not initialized".to_string(),
        )
    })?;

    let stats = stream.stats.lock().unwrap();
    Ok(Stats {
        video_kbps: stats.video_bitrate,
        audio_kbps: stats.audio_bitrate,
        rtt: stats.rtt,
        jitter: stats.jitter,
        fps: 0.0, // Will be updated in the streaming loop
    })
}

#[napi]
pub fn handle_signal(signal: String) -> napi::Result<()> {
    let stream = unsafe { STREAM.as_mut() }.ok_or_else(|| {
        napi::Error::new(
            napi::Status::GenericFailure,
            "Stream not initialized".to_string(),
        )
    })?;

    if let Some(transport) = &mut stream.transport {
        // Forward signaling messages to WebRTC transport
        // This would be implemented to handle SDP offers/answers and ICE candidates
        // from the JavaScript side
    }

    Ok(())
}

#[napi]
pub fn set_video_quality(quality: u32) -> napi::Result<()> {
    let stream = unsafe { STREAM.as_mut() }.ok_or_else(|| {
        napi::Error::new(
            napi::Status::GenericFailure,
            "Stream not initialized".to_string(),
        )
    })?;

    if let Some(video) = &mut stream.video_capture {
        // Adjust video quality settings
        // This would be implemented to adjust bitrate, resolution, etc.
    }

    Ok(())
}

#[napi]
pub fn set_audio_quality(quality: u32) -> napi::Result<()> {
    let stream = unsafe { STREAM.as_mut() }.ok_or_else(|| {
        napi::Error::new(
            napi::Status::GenericFailure,
            "Stream not initialized".to_string(),
        )
    })?;

    if let Some(audio) = &mut stream.audio_capture {
        // Adjust audio quality settings
    }

    Ok(())
}

#[napi]
pub fn is_running() -> bool {
    unsafe { STREAM.as_ref() }
        .map(|s| s.running)
        .unwrap_or(false)
}

#[napi(js_name = "StreamEvent")]
pub enum StreamEvent {
    Stats {
        video_kbps: f64,
        audio_kbps: f64,
        rtt: f64,
        jitter: f64,
        fps: f64,
    },
    Error(String),
    Connected,
    Disconnected,
    Warning(String),
}

// FFI-safe wrapper for the stream event
#[napi]
impl StreamEvent {
    #[napi(constructor)]
    pub fn new() -> Self {
        StreamEvent::Stats {
            video_kbps: 0.0,
            audio_kbps: 0.0,
            rtt: 0.0,
            jitter: 0.0,
            fps: 0.0,
        }
    }
}
