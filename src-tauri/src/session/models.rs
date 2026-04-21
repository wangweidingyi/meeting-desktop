use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    #[default]
    Idle,
    Connecting,
    Ready,
    Recording,
    Paused,
    Stopping,
    Completed,
    Error,
}

impl SessionStatus {
    pub fn as_db_value(&self) -> &'static str {
        match self {
            SessionStatus::Idle => "idle",
            SessionStatus::Connecting => "connecting",
            SessionStatus::Ready => "ready",
            SessionStatus::Recording => "recording",
            SessionStatus::Paused => "paused",
            SessionStatus::Stopping => "stopping",
            SessionStatus::Completed => "completed",
            SessionStatus::Error => "error",
        }
    }

    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            SessionStatus::Connecting
                | SessionStatus::Ready
                | SessionStatus::Recording
                | SessionStatus::Paused
                | SessionStatus::Stopping
                | SessionStatus::Error
        )
    }

    pub fn from_db_value(value: &str) -> Self {
        match value {
            "connecting" => SessionStatus::Connecting,
            "ready" => SessionStatus::Ready,
            "recording" => SessionStatus::Recording,
            "paused" => SessionStatus::Paused,
            "stopping" => SessionStatus::Stopping,
            "completed" => SessionStatus::Completed,
            "error" => SessionStatus::Error,
            _ => SessionStatus::Idle,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SessionEvent {
    ConnectRequested,
    ConnectSucceeded,
    RecordingStarted,
    PauseRequested,
    ResumeRequested,
    StopRequested,
    FlushCompleted,
    Fail,
    Reset,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MeetingRecord {
    pub id: String,
    pub title: String,
    pub status: SessionStatus,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub duration_ms: u64,
}

impl MeetingRecord {
    pub fn new(title: String) -> Self {
        let millis = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_millis())
            .unwrap_or_default();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default();

        Self {
            id: format!("meeting-{nanos}"),
            title,
            status: SessionStatus::Idle,
            started_at: millis.to_string(),
            ended_at: None,
            duration_ms: 0,
        }
    }
}
