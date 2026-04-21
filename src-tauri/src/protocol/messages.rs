use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MessageEnvelope<T> {
    pub version: String,
    pub message_id: String,
    pub correlation_id: Option<String>,
    pub client_id: String,
    pub session_id: String,
    pub seq: u64,
    pub sent_at: String,
    pub message_type: MessageType,
    pub payload: T,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MessageType {
    #[serde(rename = "session/hello")]
    SessionHello,
    #[serde(rename = "recording/start")]
    RecordingStart,
    #[serde(rename = "recording/stop")]
    RecordingStop,
    #[serde(rename = "stt_delta")]
    SttDelta,
    #[serde(rename = "summary_delta")]
    SummaryDelta,
    #[serde(rename = "error")]
    Error,
}
