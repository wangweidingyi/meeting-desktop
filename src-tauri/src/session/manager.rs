use crate::session::models::{MeetingRecord, SessionEvent};
use crate::session::state_machine::SessionStateMachine;
use crate::storage::summary_repo::SummarySnapshotRecord;
use crate::storage::transcript_repo::TranscriptSegmentRecord;
use crate::transport::mqtt_control::MqttControlConfig;

#[derive(Default)]
pub struct SessionManager {
    machine: SessionStateMachine,
    active_meeting: Option<MeetingRecord>,
}

impl SessionManager {
    pub fn create_meeting(&mut self, title: String) -> MeetingRecord {
        self.machine = SessionStateMachine::default();
        let meeting = MeetingRecord::new(title);
        self.active_meeting = Some(meeting.clone());
        meeting
    }

    pub fn activate_existing_meeting(&mut self, meeting: MeetingRecord) -> MeetingRecord {
        self.machine = SessionStateMachine::default();
        self.active_meeting = Some(meeting.clone());
        meeting
    }

    pub fn transition_active_meeting(
        &mut self,
        event: SessionEvent,
    ) -> Result<MeetingRecord, String> {
        let status = self.machine.transition(event)?;
        let meeting = self
            .active_meeting
            .as_mut()
            .ok_or_else(|| "no active meeting".to_string())?;

        meeting.status = status;
        Ok(meeting.clone())
    }

    pub fn active_meeting(&self) -> Option<&MeetingRecord> {
        self.active_meeting.as_ref()
    }

    pub fn active_meeting_id(&self) -> Option<&str> {
        self.active_meeting
            .as_ref()
            .map(|meeting| meeting.id.as_str())
    }

    pub fn transcript_segment_for_active_meeting(
        &self,
        segment_id: String,
        start_ms: u64,
        end_ms: u64,
        text: String,
        is_final: bool,
        speaker_id: Option<String>,
        revision: u64,
    ) -> Result<TranscriptSegmentRecord, String> {
        let meeting_id = self
            .active_meeting_id()
            .ok_or_else(|| "no active meeting".to_string())?;

        Ok(TranscriptSegmentRecord {
            segment_id,
            meeting_id: meeting_id.to_string(),
            start_ms,
            end_ms,
            text,
            is_final,
            speaker_id,
            revision,
        })
    }

    pub fn summary_snapshot_for_active_meeting(
        &self,
        version: u64,
        updated_at: String,
        abstract_text: String,
        key_points: Vec<String>,
        decisions: Vec<String>,
        risks: Vec<String>,
        action_items: Vec<String>,
        is_final: bool,
    ) -> Result<SummarySnapshotRecord, String> {
        let meeting_id = self
            .active_meeting_id()
            .ok_or_else(|| "no active meeting".to_string())?;

        Ok(SummarySnapshotRecord {
            meeting_id: meeting_id.to_string(),
            version,
            updated_at,
            abstract_text,
            key_points,
            decisions,
            risks,
            action_items,
            is_final,
        })
    }

    pub fn mqtt_control_config(&self, client_id: &str) -> Result<MqttControlConfig, String> {
        let meeting = self
            .active_meeting()
            .ok_or_else(|| "no active meeting".to_string())?;

        Ok(MqttControlConfig {
            client_id: client_id.to_string(),
            session_id: meeting.id.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::SessionManager;
    use crate::session::models::{MeetingRecord, SessionStatus};

    #[test]
    fn mqtt_control_config_uses_active_meeting_identity() {
        let mut manager = SessionManager::default();
        let created = manager.create_meeting("控制链路测试".to_string());

        let config = manager.mqtt_control_config("desktop-client").unwrap();

        assert_eq!(config.client_id, "desktop-client");
        assert_eq!(config.session_id, created.id);
        assert_eq!(
            manager.active_meeting().unwrap().status,
            SessionStatus::Idle
        );
    }

    #[test]
    fn activate_existing_meeting_reuses_meeting_identity() {
        let mut manager = SessionManager::default();
        let existing = MeetingRecord {
            id: "meeting-existing".to_string(),
            title: "恢复中的会议".to_string(),
            status: SessionStatus::Error,
            started_at: "1713770000".to_string(),
            ended_at: None,
            duration_ms: 123_000,
        };

        let activated = manager.activate_existing_meeting(existing.clone());

        assert_eq!(activated.id, existing.id);
        assert_eq!(manager.active_meeting_id(), Some("meeting-existing"));
        assert_eq!(manager.active_meeting().unwrap().title, "恢复中的会议");
    }
}
