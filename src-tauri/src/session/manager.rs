use crate::session::models::{MeetingRecord, SessionEvent};
use crate::session::state_machine::SessionStateMachine;

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
}
