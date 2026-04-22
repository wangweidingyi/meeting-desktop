use serde::{Deserialize, Serialize};

use crate::session::models::{MeetingRecord, SessionStatus};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionSnapshot {
    pub meeting: Option<MeetingRecord>,
    pub status: SessionStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TranscriptDeltaPayload {
    pub text: String,
    pub is_final: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SummaryDeltaPayload {
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RuntimeEvent {
    SessionUpdated(SessionSnapshot),
    TranscriptDelta(TranscriptDeltaPayload),
    SummaryDelta(SummaryDeltaPayload),
    Heartbeat { session_id: String },
    TransportError { message: String },
}
