use crate::error::{Result, SlumpError};
use bytes::Bytes;
use futures_util::{
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::mpsc,
};
use tokio_tungstenite::{
    connect_async, connect_async_with_config, tungstenite::protocol::Message, MaybeTlsStream,
    WebSocketStream,
};
use webrtc::{
    api::{
        interceptor_registry::register_default_interceptors,
        media_engine::{MediaEngine, MIME_TYPE_OPUS, MIME_TYPE_VP8},
        APIBuilder,
    },
    ice_transport::ice_server::RTCIceServer,
    interceptor::registry::Registry,
    media::{
        codec::h264::h264_errors::Error as H264Error,
        sample::Sample,
        track::track_local::track_local_static_rtp::TrackLocalStaticRTP,
    },
    peer_connection::{
        configuration::RTCConfiguration,
        peer_connection_state::RTCPeerConnectionState,
        sdp::session_description::RTCSessionDescription,
        RTCPeerConnection,
    },
    rtp_transceiver::rtp_codec::{
        RTCRtpCodecCapability, RTCRtpCodecParameters, RTPCodecType, RTCRtpCodecParametersParameters,
    },
    track::track_local::track_local_static_rtp::TrackLocalStaticRTPOptions,
    util::Unmarshal,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IceCandidate {
    pub candidate: String,
    pub sdp_mid: Option<String>,
    pub sdp_m_line_index: Option<u16>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum SignalMessage {
    Offer { sdp: String },
    Answer { sdp: String },
    Ice { candidate: IceCandidate },
    Error(String),
}

pub struct WebRTCTransport {
    peer_connection: Arc<RTCPeerConnection>,
    video_track: Arc<TrackLocalStaticRTP>,
    audio_track: Arc<TrackLocalStaticRTP>,
    ws_sender: mpsc::UnboundedSender<Message>,
    last_stats: Arc<Mutex<Option<Stats>>>,
    last_ping: Arc<Mutex<Instant>>,
}

#[derive(Debug, Clone)]
pub struct Stats {
    pub timestamp: Instant,
    pub bytes_sent: u64,
    pub packets_sent: u64,
    pub rtt: f64,
    pub jitter: f64,
    pub bitrate: f64,
}

impl WebRTCTransport {
    pub async fn new(
        stun_servers: Vec<String>,
        turn_servers: Vec<(String, Option<String>, Option<String>)>,
    ) -> Result<Self> {
        // Configure WebRTC
        let mut media_engine = MediaEngine::default();
        media_engine.register_default_codecs()?;
        
        // Configure VP8 and Opus codecs
        media_engine.register_codec(
            RTCRtpCodecParameters {
                capability: RTCRtpCodecCapability {
                    mime_type: MIME_TYPE_VP8.to_owned(),
                    clock_rate: 90000,
                    channels: 0,
                    sdp_fmtp_line: "profile-level-id=42e01f;level-asymmetry-allowed=1".to_owned(),
                    rtcp_feedback: vec![],
                },
                payload_type: 96,
                ..Default::default()
            },
            RTPCodecType::Video,
        )?;

        media_engine.register_codec(
            RTCRtpCodecParameters {
                capability: RTCRtpCodecCapability {
                    mime_type: MIME_TYPE_OPUS.to_owned(),
                    clock_rate: 48000,
                    channels: 2,
                    sdp_fmtp_line: "minptime=10;useinbandfec=1".to_owned(),
                    rtcp_feedback: vec![],
                },
                payload_type: 111,
                ..Default::default()
            },
            RTPCodecType::Audio,
        )?;

        let mut registry = Registry::new();
        register_default_interceptors(&mut registry, &mut media_engine)?;

        // Configure ICE servers
        let mut ice_servers = vec![];
        
        for stun in stun_servers {
            ice_servers.push(RTCIceServer {
                urls: vec![stun],
                username: String::new(),
                credential: String::new(),
                credential_type: webrtc::ice_transport::ice_credential_type::RTCIceCredentialType::Unspecified,
            });
        }
        
        for (url, username, credential) in turn_servers {
            ice_servers.push(RTCIceServer {
                urls: vec![url],
                username: username.unwrap_or_default(),
                credential: credential.unwrap_or_default(),
                credential_type: webrtc::ice_transport::ice_credential_type::RTCIceCredentialType::Password,
            });
        }

        let config = RTCConfiguration {
            ice_servers,
            ..Default::default()
        };

        let api = APIBuilder::new()
            .with_media_engine(media_engine)
            .with_interceptor_registry(registry)
            .build();

        let peer_connection = Arc::new(api.new_peer_connection(config).await?);

        // Create video track
        let video_track = Arc::new(
            TrackLocalStaticRTP::new(
                RTCRtpCodecCapability {
                    mime_type: MIME_TYPE_VP8.to_owned(),
                    clock_rate: 90000,
                    channels: 0,
                    sdp_fmtp_line: "profile-level-id=42e01f;level-asymmetry-allowed=1".to_owned(),
                    rtcp_feedback: vec![],
                },
                "video".to_owned(),
                "slump-video".to_owned(),
            )
            .map_err(|e| SlumpError::Webrtc(e.to_string()))?,
        );

        // Create audio track
        let audio_track = Arc::new(
            TrackLocalStaticRTP::new(
                RTCRtpCodecCapability {
                    mime_type: MIME_TYPE_OPUS.to_owned(),
                    clock_rate: 48000,
                    channels: 2,
                    sdp_fmtp_line: "minptime=10;useinbandfec=1".to_owned(),
                    rtcp_feedback: vec![],
                },
                "audio".to_owned(),
                "slump-audio".to_owned(),
            )
            .map_err(|e| SlumpError::Webrtc(e.to_string()))?,
        );

        // Add tracks to peer connection
        let rtp_sender = peer_connection
            .add_track(Arc::clone(&video_track) as Arc<_>)
            .await
            .map_err(|e| SlumpError::Webrtc(e.to_string()))?;

        let _rtp_sender_audio = peer_connection
            .add_track(Arc::clone(&audio_track) as Arc<_>)
            .await
            .map_err(|e| SlumpError::Webrtc(e.to_string()))?;

        // Setup data channel for control messages
        let data_channel = peer_connection
            .create_data_channel("control", None)
            .map_err(|e| SlumpError::Webrtc(e.to_string()))?;

        // Setup stats collection
        let last_stats = Arc::new(Mutex::new(None));
        let last_stats_clone = Arc::clone(&last_stats);
        
        // Setup ping/pong for connection monitoring
        let last_ping = Arc::new(Mutex::new(Instant::now()));
        
        // Create WebSocket channel for signaling
        let (ws_sender, mut ws_receiver) = mpsc::unbounded_channel::<Message>();
        
        // Spawn a task to handle incoming WebSocket messages
        let peer_connection_clone = Arc::clone(&peer_connection);
        tokio::spawn(async move {
            while let Some(message) = ws_receiver.recv().await {
                if let Ok(text) = message.into_text() {
                    if let Ok(signal) = serde_json::from_str::<SignalMessage>(&text) {
                        match signal {
                            SignalMessage::Answer { sdp } => {
                                if let Err(e) = peer_connection_clone.set_remote_description(
                                    RTCSessionDescription::answer(sdp).map_err(|e| SlumpError::Webrtc(e.to_string()))?
                                ).await {
                                    log::error!("Failed to set remote description: {}", e);
                                }
                            },
                            SignalMessage::Ice { candidate } => {
                                if let Err(e) = peer_connection_clone.add_ice_candidate(candidate).await {
                                    log::error!("Failed to add ICE candidate: {}", e);
                                }
                            },
                            _ => {}
                        }
                    }
                }
            }
            Ok::<(), anyhow::Error>(())
        });

        Ok(Self {
            peer_connection,
            video_track,
            audio_track,
            ws_sender,
            last_stats,
            last_ping,
        })
    }

    pub async fn send_video_frame(&self, frame: &[u8], timestamp: u32) -> Result<()> {
        self.video_track.write_rtp(&frame, timestamp, None)?;
        Ok(())
    }

    pub async fn send_audio_frame(&self, frame: &[u8], timestamp: u32) -> Result<()> {
        self.audio_track.write_rtp(&frame, timestamp, None)?;
        Ok(())
    }

    pub fn get_stats(&self) -> Option<Stats> {
        self.last_stats.lock().unwrap().clone()
    }

    pub fn is_connected(&self) -> bool {
        self.last_ping.lock().unwrap().elapsed() < Duration::from_secs(5)
    }
}

impl Drop for WebRTCTransport {
    fn drop(&mut self) {
        let pc = Arc::clone(&self.peer_connection);
        tokio::spawn(async move {
            let _ = pc.close().await;
        });
    }
}
