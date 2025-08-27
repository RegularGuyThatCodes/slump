use crate::error::{Result, SlumpError};
use ffmpeg_next::{
    codec,
    format::pixel::Pixel,
    software::scaling,
    util::frame,
    Dictionary,
    Frame,
};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

pub struct VideoCapture {
    input_ctx: ffmpeg_next::format::context::Input,
    stream_index: usize,
    decoder: codec::decoder::Video,
    scaler: scaling::Context,
    last_frame: Option<Frame>,
    last_pts: Option<i64>,
    frame_rate: f64,
    frame_count: u64,
    start_time: Instant,
}

impl VideoCapture {
    pub fn new(display_index: usize, width: u32, height: u32) -> Result<Self> {
        ffmpeg_next::init().map_err(|e| SlumpError::Init(e.to_string()))?;

        // Setup display capture
        let input_format = if cfg!(windows) {
            "gdigrab"
        } else if cfg!(target_os = "macos") {
            "avfoundation"
        } else {
            "x11grab"
        };

        let input_url = if cfg!(windows) {
            format!("desktop")
        } else if cfg!(target_os = "macos") {
            format!("1:0")
        } else {
            format!(":0.0+0,0")
        };

        let mut options = Dictionary::new();
        options.set("framerate", "120");
        options.set("video_size", &format!("{}x{}", width, height));
        options.set("draw_mouse", "0");

        let mut input_ctx = ffmpeg_next::format::input_with_dictionary(
            &input_format,
            &input_url,
            options,
        )?;

        let stream = input_ctx
            .streams()
            .best(ffmpeg_next::media::Type::Video)
            .ok_or_else(|| SlumpError::Init("No video stream found".into()))?;

        let stream_index = stream.index();
        let context_decoder = ffmpeg_next::codec::context::Context::from_parameters(stream.parameters())?;
        let mut decoder = context_decoder.decoder().video()?;

        decoder.set_threading(ffmpeg_next::config::Config {
            thread_type: ffmpeg_next::threading::Type::Frame,
            ..Default::default()
        });

        let decoder = decoder.open()?;
        let scaler = scaling::Context::get(
            decoder.format(),
            decoder.width(),
            decoder.height(),
            ffmpeg_next::format::pixel::Pixel::NV12,
            width,
            height,
            scaling::Flags::BILINEAR,
        )?;

        Ok(Self {
            input_ctx,
            stream_index,
            decoder,
            scaler,
            last_frame: None,
            last_pts: None,
            frame_rate: 90.0,
            frame_count: 0,
            start_time: Instant::now(),
        })
    }

    pub fn capture_frame(&mut self) -> Result<Option<Frame>> {
        let mut packet = match self.input_ctx.packets().next() {
            Some((_, packet)) => packet,
            None => return Ok(None),
        };

        if packet.stream() != self.stream_index {
            return Ok(None);
        }

        self.decoder.send_packet(&packet)?;
        
        let mut decoded = Frame::empty();
        if self.decoder.receive_frame(&mut decoded).is_ok() {
            let mut scaled = Frame::empty();
            self.scaler.run(&decoded, &mut scaled)?;
            self.last_frame = Some(scaled);
            self.frame_count += 1;
            self.last_pts = decoded.pts().map(|p| p as i64);
            
            // Calculate actual frame rate
            let elapsed = self.start_time.elapsed();
            if elapsed.as_secs() > 0 {
                self.frame_rate = self.frame_count as f64 / elapsed.as_secs_f64();
            }
            
            Ok(Some(decoded))
        } else {
            Ok(None)
        }
    }

    pub fn get_frame_rate(&self) -> f64 {
        self.frame_rate
    }

    pub fn get_last_frame(&self) -> Option<&Frame> {
        self.last_frame.as_ref()
    }
}

impl Drop for VideoCapture {
    fn drop(&mut self) {
        let _ = self.decoder.send_eof();
    }
}
