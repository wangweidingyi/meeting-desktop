use meeting_lib::backend_sync::{InMemoryMeetingSync, MeetingSync};
use meeting_lib::session::manager::SessionManager;
use meeting_lib::session::models::{SessionEvent, SessionStatus};

#[test]
fn session_lifecycle_builds_completed_meeting_and_persists_final_artifacts_in_memory_sync() {
    let sync = InMemoryMeetingSync::default();
    let mut manager = SessionManager::default();

    let mut meeting = manager.create_meeting("桌面端 smoke".to_string());

    for event in [
        SessionEvent::ConnectRequested,
        SessionEvent::ConnectSucceeded,
        SessionEvent::RecordingStarted,
        SessionEvent::StopRequested,
        SessionEvent::FlushCompleted,
    ] {
        meeting = manager
            .transition_active_meeting(event)
            .expect("transition meeting");
    }

    let transcript = manager
        .transcript_segment_for_active_meeting(
            "segment-1".to_string(),
            0,
            2_000,
            "主持人：确认 mixed 单流与 WAV 保留策略。".to_string(),
            true,
            None,
            2,
        )
        .expect("build transcript");
    let summary = manager
        .summary_snapshot_for_active_meeting(
            1,
            "2026-04-22T00:00:00Z".to_string(),
            "会议完成了首版客户端与服务端联调 smoke。".to_string(),
            vec!["Rust 负责主控与持久化".to_string()],
            vec!["首版音频链路采用 MQTT + UDP".to_string()],
            vec!["真实 STT 尚未接入".to_string()],
            vec!["继续补充端到端联调".to_string()],
            true,
        )
        .expect("build summary");

    sync.upsert_transcript_segment(&transcript)
        .expect("persist transcript");
    sync.upsert_summary_snapshot(&summary)
        .expect("persist summary");

    let persisted_transcript = sync.transcript_segments(&meeting.id);
    let persisted_summary = sync
        .latest_summary(&meeting.id)
        .expect("summary should exist");

    assert_eq!(meeting.status, SessionStatus::Completed);
    assert_eq!(persisted_transcript.len(), 1);
    assert!(persisted_transcript[0].is_final);
    assert!(persisted_summary.is_final);
    assert_eq!(
        persisted_summary.action_items,
        vec!["继续补充端到端联调".to_string()]
    );
}
