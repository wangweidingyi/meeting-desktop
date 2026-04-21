pub fn control_topic(client_id: &str, session_id: &str) -> String {
    format!("meetings/{client_id}/session/{session_id}/control")
}

pub fn control_reply_topic(client_id: &str, session_id: &str) -> String {
    format!("meetings/{client_id}/session/{session_id}/control/reply")
}

pub fn events_topic(client_id: &str, session_id: &str) -> String {
    format!("meetings/{client_id}/session/{session_id}/events")
}

#[cfg(test)]
mod tests {
    use super::control_topic;

    #[test]
    fn protocol_topics_build_expected_paths() {
        assert_eq!(
            control_topic("client-a", "session-1"),
            "meetings/client-a/session/session-1/control"
        );
    }
}
