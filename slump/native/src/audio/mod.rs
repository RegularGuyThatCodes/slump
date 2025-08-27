use crate::error::{Result, SlumpError};
use ffmpeg_next::{
    codec,
    format::sample::Sample,
    frame,
    util::frame::audio::Audio,
    Dictionary,
};
use ringbuf::{HeapRb, Rb};
use std::{
    sync::{Arc, Mutex},
    time::Instant,
};

const SAMPLE_RATE: i32 = 48000;
const CHANNELS: u16 = 2; // Stereo
const FRAME_SIZE: usize = 960; // 20ms at 48kHz

pub struct AudioCapture {
    input_ctx: ffmpeg_next::format::context::Input,
    stream_index: usize,
    decoder: codec::decoder::Audio,
    resampler: Option<ffmpeg_next::software::resampling::Context>,
    ring_buffer: Arc<Mutex<HeapRb<f32>>>,
    start_time: Instant,
}

impl AudioCapture {
    pub fn new() -> Result<Self> {
        let input_format = if cfg!(windows) {
            "dshow"
        } else if cfg!(target_os = "macos") {
            "avfoundation"
        } else {
            "pulse"
        };

        let input_url = if cfg!(windows) {
            "audio=Microphone"
        } else if cfg!(target_os = "macos") {
            ":0"
        } else {
            "default"
        };

        let mut options = Dictionary::new();
        options.set("sample_rate", &SAMPLE_RATE.to_string());
        options.set("channels", &CHANNELS.to_string());
        options.set("threads", "0");

        let mut input_ctx = ffmpeg_next::format::input_with_dictionary(
            &format!("{}", input_format),
            input_url,
            options,
        )?;

        let stream = input_ctx
            .streams()
            .best(ffmpeg_next::media::Type::Audio)
            .ok_or_else(|| SlumpError::Audio("No audio stream found".into()))?;

        let stream_index = stream.index();
        let context_decoder = ffmpeg_next::codec::context::Context::from_parameters(stream.parameters())?;
        let mut decoder = context_decoder.decoder().audio()?;
        
        // Configure decoder
        decoder.set_threading(ffmpeg_next::config::Config {
            thread_type: ffmpeg_next::threading::Type::Frame,
            ..Default::default()
        });
        
        let decoder = decoder.open()?;
        
        // Create resampler if needed
        let resampler = if decoder.format() != ffmpeg_next::format::Sample::FLTP || 
                          decoder.rate() != SAMPLE_RATE || 
                          decoder.channel_layout().channels() != CHANNELS {
            Some(
                ffmpeg_next::software::resampling::Context::get(
                    decoder.format(),
                    decoder.channel_layout(),
                    decoder.rate(),
                    ffmpeg_next::format::Sample::FLTP,
                    ffmpeg_next::channel_layout::ChannelLayout::STEREO,
                    SAMPLE_RATE,
                    ffmpeg_next::software::resampling::Flag::FAST_INTEGER,
                )?
            )
        } else {
            None
        };

        // Ring buffer for audio data (1 second of audio)
        let ring_buffer = Arc::new(Mutex::new(HeapRb::<f32>::new((SAMPLE_RATE * 2) as usize)));

        Ok(Self {
            input_ctx,
            stream_index,
            decoder,
            resampler,
            ring_buffer,
            start_time: Instant::now(),
        })
    }

    pub fn capture_audio(&mut self) -> Result<()> {
        let mut packet = match self.input_ctx.packets().next() {
            Some((_, packet)) => packet,
            None => return Ok(()),
        };

        if packet.stream() != self.stream_index {
            return Ok(());
        }

        self.decoder.send_packet(&packet)?;
        
        let mut decoded = frame::Audio::empty();
        while self.decoder.receive_frame(&mut decoded).is_ok() {
            let mut resampled = frame::Audio::empty();
            
            // Resample if needed
            let processed = if let Some(ref mut resampler) = self.resampler {
                resampler.run(&decoded, &mut resampled)?;
                &resampled
            } else {
                &decoded
            };

            // Convert to interleaved f32 and push to ring buffer
            let data = processed.data(0);
            let samples = unsafe {
                std::slice::from_raw_parts(
                    data.as_ptr() as *const f32,
                    data.len() / std::mem::size_of::<f32>(),
                )
            };
            
            let mut rb = self.ring_buffer.lock().unwrap();
            for &sample in samples {
                let _ = rb.push(sample);
            }
        }
        
        Ok(())
    }

    pub fn read_audio(&self, buffer: &mut [f32]) -> usize {
        let mut rb = self.ring_buffer.lock().unwrap();
        let count = buffer.len().min(rb.len());
        
        for i in 0..count {
            if let Some(sample) = rb.pop() {
                buffer[i] = sample;
            } else {
                return i;
            }
        }
        
        count
    }
}

impl Drop for AudioCapture {
    fn drop(&mut self) {
        let _ = self.decoder.send_eof();
    }
}
