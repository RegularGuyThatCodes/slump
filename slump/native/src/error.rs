use std::fmt;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SlumpError {
    #[error("FFmpeg error: {0}")]
    Ffmpeg(String),
    
    #[error("WebRTC error: {0}")]
    Webrtc(String),
    
    #[error("Audio capture error: {0}")]
    Audio(String),
    
    #[error("Video capture error: {0}")]
    Video(String),
    
    #[error("Network error: {0}")]
    Network(String),
    
    #[error("Initialization error: {0}")]
    Init(String),
    
    #[error("Not implemented: {0}")]
    NotImplemented(String),
}

impl From<ffmpeg_next::Error> for SlumpError {
    fn from(err: ffmpeg_next::Error) -> Self {
        SlumpError::Ffmpeg(err.to_string())
    }
}

pub type Result<T> = std::result::Result<T, SlumpError>;
