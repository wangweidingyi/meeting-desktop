use crate::session::models::{SessionEvent, SessionStatus};

#[derive(Debug, Clone, Default)]
pub struct SessionStateMachine {
    current: SessionStatus,
}

impl SessionStateMachine {
    pub fn current(&self) -> SessionStatus {
        self.current.clone()
    }

    pub fn transition(&mut self, event: SessionEvent) -> Result<SessionStatus, String> {
        let next = match (&self.current, event) {
            (SessionStatus::Idle, SessionEvent::ConnectRequested) => SessionStatus::Connecting,
            (SessionStatus::Connecting, SessionEvent::ConnectSucceeded) => SessionStatus::Ready,
            (SessionStatus::Ready, SessionEvent::RecordingStarted) => SessionStatus::Recording,
            (SessionStatus::Recording, SessionEvent::PauseRequested) => SessionStatus::Paused,
            (SessionStatus::Paused, SessionEvent::ResumeRequested) => SessionStatus::Recording,
            (SessionStatus::Recording, SessionEvent::StopRequested)
            | (SessionStatus::Paused, SessionEvent::StopRequested) => SessionStatus::Stopping,
            (SessionStatus::Stopping, SessionEvent::FlushCompleted) => SessionStatus::Completed,
            (_, SessionEvent::Fail) => SessionStatus::Error,
            (_, SessionEvent::Reset) => SessionStatus::Idle,
            (current, event) => {
                return Err(format!(
                    "invalid transition from {current:?} with event {event:?}"
                ))
            }
        };

        self.current = next.clone();
        Ok(next)
    }
}

#[cfg(test)]
mod tests {
    use super::SessionStateMachine;
    use crate::session::models::{SessionEvent, SessionStatus};

    #[test]
    fn session_state_defaults_to_idle() {
        let machine = SessionStateMachine::default();
        assert_eq!(machine.current(), SessionStatus::Idle);
    }

    #[test]
    fn start_pause_resume_stop_follows_valid_transitions() {
        let mut machine = SessionStateMachine::default();

        machine.transition(SessionEvent::ConnectRequested).unwrap();
        machine.transition(SessionEvent::ConnectSucceeded).unwrap();
        machine.transition(SessionEvent::RecordingStarted).unwrap();
        machine.transition(SessionEvent::PauseRequested).unwrap();
        machine.transition(SessionEvent::ResumeRequested).unwrap();
        let final_state = machine.transition(SessionEvent::StopRequested).unwrap();

        assert_eq!(final_state, SessionStatus::Stopping);
    }
}
