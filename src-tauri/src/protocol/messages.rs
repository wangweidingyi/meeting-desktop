use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AudioFormat {
    pub encoding: String,
    pub sample_rate: u32,
    pub channels: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SessionHelloPayload {
    pub audio: AudioFormat,
    pub transport: TransportSelection,
    pub features: FeatureFlags,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TransportSelection {
    pub control: String,
    pub audio: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FeatureFlags {
    pub realtime_transcript: bool,
    pub realtime_summary: bool,
    pub action_items: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ControlAckPayload {
    pub accepted: bool,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RecordingStatePayload {
    pub state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MessageEnvelope<T> {
    pub version: String,
    pub message_id: String,
    pub correlation_id: Option<String>,
    pub client_id: String,
    pub session_id: String,
    pub seq: u64,
    pub sent_at: String,
    #[serde(rename = "type")]
    pub message_type: MessageType,
    pub payload: T,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MessageType {
    #[serde(rename = "session/hello")]
    SessionHello,
    #[serde(rename = "session/resume")]
    SessionResume,
    #[serde(rename = "session/close")]
    SessionClose,
    #[serde(rename = "recording/start")]
    RecordingStart,
    #[serde(rename = "recording/pause")]
    RecordingPause,
    #[serde(rename = "recording/resume")]
    RecordingResume,
    #[serde(rename = "recording/stop")]
    RecordingStop,
    #[serde(rename = "recording_started")]
    RecordingStarted,
    #[serde(rename = "recording_paused")]
    RecordingPaused,
    #[serde(rename = "recording_resumed")]
    RecordingResumed,
    #[serde(rename = "recording_stopped")]
    RecordingStopped,
    #[serde(rename = "stt_delta")]
    SttDelta,
    #[serde(rename = "stt_final")]
    SttFinal,
    #[serde(rename = "summary_delta")]
    SummaryDelta,
    #[serde(rename = "summary_final")]
    SummaryFinal,
    #[serde(rename = "action_item_delta")]
    ActionItemDelta,
    #[serde(rename = "action_item_final")]
    ActionItemFinal,
    #[serde(rename = "heartbeat")]
    Heartbeat,
    #[serde(rename = "ack")]
    Ack,
    #[serde(rename = "error")]
    Error,
}
