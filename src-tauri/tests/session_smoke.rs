use meeting_lib::session::manager::SessionManager;
use meeting_lib::session::models::{SessionEvent, SessionStatus};
use meeting_lib::storage::db::Database;
use meeting_lib::storage::meetings_repo::MeetingsRepo;
use meeting_lib::storage::summary_repo::SummaryRepo;
use meeting_lib::storage::transcript_repo::TranscriptRepo;

#[test]
fn session_lifecycle_persists_completed_meeting_and_final_artifacts() {
    let database = Database::open_in_memory().expect("open in memory database");
    let mut manager = SessionManager::default();

    let mut meeting = manager.create_meeting("桌面端 smoke".to_string());
    database
        .with_connection(|connection| MeetingsRepo::insert(connection, &meeting))
        .expect("insert meeting");

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

    database
        .with_connection(|connection| {
            MeetingsRepo::upsert(connection, &meeting)?;
            TranscriptRepo::upsert(connection, &transcript)?;
            SummaryRepo::upsert_snapshot(connection, &summary)
        })
        .expect("persist final records");

    let persisted_meeting = database
        .with_connection(|connection| MeetingsRepo::find_by_id(connection, &meeting.id))
        .expect("query meeting")
        .expect("meeting should exist");
    let persisted_transcript = database
        .with_connection(|connection| TranscriptRepo::list_by_meeting(connection, &meeting.id))
        .expect("query transcript");
    let persisted_summary = database
        .with_connection(|connection| SummaryRepo::latest_snapshot(connection, &meeting.id))
        .expect("query summary")
        .expect("summary should exist");

    assert_eq!(persisted_meeting.status, SessionStatus::Completed);
    assert_eq!(persisted_transcript.len(), 1);
    assert!(persisted_transcript[0].is_final);
    assert!(persisted_summary.is_final);
    assert_eq!(
        persisted_summary.action_items,
        vec!["继续补充端到端联调".to_string()]
    );
}
