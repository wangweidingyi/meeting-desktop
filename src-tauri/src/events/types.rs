use serde::{Deserialize, Serialize};

use crate::session::models::{MeetingRecord, SessionStatus};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TransportConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AudioUplinkState {
    Idle,
    WaitingForAudio,
    Replaying,
    Streaming,
    Paused,
    Stopped,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionSnapshot {
    pub meeting: Option<MeetingRecord>,
    pub status: SessionStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TranscriptDeltaPayload {
    pub session_id: String,
    pub segment_id: String,
    pub start_ms: u64,
    pub end_ms: u64,
    pub text: String,
    pub is_final: bool,
    pub speaker_id: Option<String>,
    pub revision: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SummaryDeltaPayload {
    pub session_id: String,
    pub version: u64,
    pub updated_at: String,
    pub abstract_text: String,
    pub key_points: Vec<String>,
    pub decisions: Vec<String>,
    pub risks: Vec<String>,
    pub action_items: Vec<String>,
    pub is_final: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ActionItemsDeltaPayload {
    pub session_id: String,
    pub version: u64,
    pub updated_at: String,
    pub items: Vec<String>,
    pub is_final: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TransportStatePayload {
    pub session_id: String,
    pub state: TransportConnectionState,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeDiagnosticsPayload {
    pub session_id: String,
    pub audio_target_addr: String,
    pub audio_uplink_state: AudioUplinkState,
    pub last_uploaded_mixed_ms: u64,
    pub last_chunk_sequence: Option<u64>,
    pub last_chunk_sent_at: Option<String>,
    pub replay_from_ms: Option<u64>,
    pub replay_until_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RuntimeEvent {
    SessionUpdated(SessionSnapshot),
    TranscriptDelta(TranscriptDeltaPayload),
    SummaryDelta(SummaryDeltaPayload),
    ActionItemsDelta(ActionItemsDeltaPayload),
    TransportStateChanged(TransportStatePayload),
    RuntimeDiagnosticsUpdated(RuntimeDiagnosticsPayload),
    Heartbeat { session_id: String },
    TransportError { message: String },
}
