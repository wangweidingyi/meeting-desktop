use serde::{Deserialize, Serialize};

use crate::session::models::{MeetingRecord, SessionStatus};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionSnapshot {
    pub meeting: Option<MeetingRecord>,
    pub status: SessionStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RuntimeEvent {
    SessionUpdated(SessionSnapshot),
}
