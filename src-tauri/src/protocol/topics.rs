pub fn control_topic(client_id: &str, session_id: &str) -> String {
    format!("meetings/{client_id}/session/{session_id}/control")
}

pub fn control_reply_topic(client_id: &str, session_id: &str) -> String {
    format!("meetings/{client_id}/session/{session_id}/control/reply")
}

pub fn events_topic(client_id: &str, session_id: &str) -> String {
    format!("meetings/{client_id}/session/{session_id}/events")
}

pub fn stt_topic(client_id: &str, session_id: &str) -> String {
    format!("meetings/{client_id}/session/{session_id}/stt")
}

pub fn summary_topic(client_id: &str, session_id: &str) -> String {
    format!("meetings/{client_id}/session/{session_id}/summary")
}

pub fn action_items_topic(client_id: &str, session_id: &str) -> String {
    format!("meetings/{client_id}/session/{session_id}/action-items")
}

#[cfg(test)]
mod tests {
    use super::{
        action_items_topic, control_reply_topic, control_topic, events_topic, stt_topic,
        summary_topic,
    };

    #[test]
    fn protocol_topics_build_expected_paths() {
        assert_eq!(
            control_topic("client-a", "session-1"),
            "meetings/client-a/session/session-1/control"
        );
        assert_eq!(
            control_reply_topic("client-a", "session-1"),
            "meetings/client-a/session/session-1/control/reply"
        );
        assert_eq!(
            events_topic("client-a", "session-1"),
            "meetings/client-a/session/session-1/events"
        );
        assert_eq!(
            stt_topic("client-a", "session-1"),
            "meetings/client-a/session/session-1/stt"
        );
        assert_eq!(
            summary_topic("client-a", "session-1"),
            "meetings/client-a/session/session-1/summary"
        );
        assert_eq!(
            action_items_topic("client-a", "session-1"),
            "meetings/client-a/session/session-1/action-items"
        );
    }
}
