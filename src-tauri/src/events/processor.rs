use tauri::{AppHandle, Emitter};

use crate::events::types::RuntimeEvent;
use crate::storage::db::Database;
use crate::storage::summary_repo::{SummaryRepo, SummarySnapshotRecord};
use crate::storage::transcript_repo::{TranscriptRepo, TranscriptSegmentRecord};

pub const DESKTOP_EVENT_SESSION_UPDATED: &str = "meeting://session-updated";
pub const DESKTOP_EVENT_TRANSCRIPT_DELTA: &str = "meeting://transcript-delta";
pub const DESKTOP_EVENT_SUMMARY_DELTA: &str = "meeting://summary-delta";
pub const DESKTOP_EVENT_ACTION_ITEMS_DELTA: &str = "meeting://action-items-delta";
pub const DESKTOP_EVENT_TRANSPORT_STATE: &str = "meeting://transport-state";
pub const DESKTOP_EVENT_TRANSPORT_ERROR: &str = "meeting://transport-error";

pub fn process_runtime_event(database: &Database, event: &RuntimeEvent) -> Result<(), String> {
    match event {
        RuntimeEvent::TranscriptDelta(payload) => database
            .with_connection(|connection| {
                TranscriptRepo::upsert(
                    connection,
                    &TranscriptSegmentRecord {
                        segment_id: payload.segment_id.clone(),
                        meeting_id: payload.session_id.clone(),
                        start_ms: payload.start_ms,
                        end_ms: payload.end_ms,
                        text: payload.text.clone(),
                        is_final: payload.is_final,
                        speaker_id: payload.speaker_id.clone(),
                        revision: payload.revision,
                    },
                )
            })
            .map_err(|error| error.to_string()),
        RuntimeEvent::SummaryDelta(payload) => database
            .with_connection(|connection| {
                SummaryRepo::upsert_snapshot(
                    connection,
                    &SummarySnapshotRecord {
                        meeting_id: payload.session_id.clone(),
                        version: payload.version,
                        updated_at: payload.updated_at.clone(),
                        abstract_text: payload.abstract_text.clone(),
                        key_points: payload.key_points.clone(),
                        decisions: payload.decisions.clone(),
                        risks: payload.risks.clone(),
                        action_items: payload.action_items.clone(),
                        is_final: payload.is_final,
                    },
                )
            })
            .map_err(|error| error.to_string()),
        RuntimeEvent::ActionItemsDelta(payload) => database
            .with_connection(|connection| {
                let existing = SummaryRepo::latest_snapshot(connection, &payload.session_id)?;
                let snapshot = if let Some(existing) = existing {
                    SummarySnapshotRecord {
                        meeting_id: existing.meeting_id,
                        version: payload.version.max(existing.version),
                        updated_at: payload.updated_at.clone(),
                        abstract_text: existing.abstract_text,
                        key_points: existing.key_points,
                        decisions: existing.decisions,
                        risks: existing.risks,
                        action_items: payload.items.clone(),
                        is_final: payload.is_final || existing.is_final,
                    }
                } else {
                    SummarySnapshotRecord {
                        meeting_id: payload.session_id.clone(),
                        version: payload.version,
                        updated_at: payload.updated_at.clone(),
                        abstract_text: String::new(),
                        key_points: vec![],
                        decisions: vec![],
                        risks: vec![],
                        action_items: payload.items.clone(),
                        is_final: payload.is_final,
                    }
                };

                SummaryRepo::upsert_snapshot(connection, &snapshot)
            })
            .map_err(|error| error.to_string()),
        _ => Ok(()),
    }
}

pub fn emit_runtime_event(app_handle: &AppHandle, event: &RuntimeEvent) -> Result<(), String> {
    match event {
        RuntimeEvent::SessionUpdated(payload) => app_handle
            .emit(DESKTOP_EVENT_SESSION_UPDATED, payload)
            .map_err(|error| error.to_string()),
        RuntimeEvent::TranscriptDelta(payload) => app_handle
            .emit(DESKTOP_EVENT_TRANSCRIPT_DELTA, payload)
            .map_err(|error| error.to_string()),
        RuntimeEvent::SummaryDelta(payload) => app_handle
            .emit(DESKTOP_EVENT_SUMMARY_DELTA, payload)
            .map_err(|error| error.to_string()),
        RuntimeEvent::ActionItemsDelta(payload) => app_handle
            .emit(DESKTOP_EVENT_ACTION_ITEMS_DELTA, payload)
            .map_err(|error| error.to_string()),
        RuntimeEvent::TransportStateChanged(payload) => app_handle
            .emit(DESKTOP_EVENT_TRANSPORT_STATE, payload)
            .map_err(|error| error.to_string()),
        RuntimeEvent::TransportError { message } => app_handle
            .emit(DESKTOP_EVENT_TRANSPORT_ERROR, message)
            .map_err(|error| error.to_string()),
        RuntimeEvent::Heartbeat { .. } => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use crate::events::types::{
        ActionItemsDeltaPayload, RuntimeEvent, SummaryDeltaPayload, TranscriptDeltaPayload,
    };
    use crate::storage::db::Database;
    use crate::storage::summary_repo::SummaryRepo;
    use crate::storage::transcript_repo::TranscriptRepo;

    use super::process_runtime_event;

    #[test]
    fn process_runtime_event_persists_transcript_and_summary_payloads() {
        let database = Database::open_in_memory().unwrap();

        process_runtime_event(
            &database,
            &RuntimeEvent::TranscriptDelta(TranscriptDeltaPayload {
                session_id: "meeting-1".to_string(),
                segment_id: "segment-1".to_string(),
                start_ms: 0,
                end_ms: 1200,
                text: "先落一条实时转写".to_string(),
                is_final: false,
                speaker_id: None,
                revision: 1,
            }),
        )
        .unwrap();

        process_runtime_event(
            &database,
            &RuntimeEvent::SummaryDelta(SummaryDeltaPayload {
                session_id: "meeting-1".to_string(),
                version: 2,
                updated_at: "2026-04-22T12:00:00Z".to_string(),
                abstract_text: "纪要已经刷新".to_string(),
                key_points: vec!["实时事件开始写库".to_string()],
                decisions: vec!["控制通道接 MQTT".to_string()],
                risks: vec!["仍需补 action-item 独立事件".to_string()],
                action_items: vec!["继续打通前端展示".to_string()],
                is_final: false,
            }),
        )
        .unwrap();

        let transcript = database
            .with_connection(|connection| {
                TranscriptRepo::find_by_segment_id(connection, "segment-1")
            })
            .unwrap()
            .unwrap();
        let summary = database
            .with_connection(|connection| SummaryRepo::latest_snapshot(connection, "meeting-1"))
            .unwrap()
            .unwrap();

        assert_eq!(transcript.text, "先落一条实时转写");
        assert_eq!(summary.version, 2);
        assert_eq!(summary.action_items, vec!["继续打通前端展示".to_string()]);
    }

    #[test]
    fn process_runtime_event_merges_action_items_delta_into_latest_summary() {
        let database = Database::open_in_memory().unwrap();

        process_runtime_event(
            &database,
            &RuntimeEvent::SummaryDelta(SummaryDeltaPayload {
                session_id: "meeting-2".to_string(),
                version: 1,
                updated_at: "2026-04-22T12:00:00Z".to_string(),
                abstract_text: "先有纪要主体".to_string(),
                key_points: vec!["关键要点".to_string()],
                decisions: vec!["先接通控制链路".to_string()],
                risks: vec!["仍需补 action-item 事件".to_string()],
                action_items: vec![],
                is_final: false,
            }),
        )
        .unwrap();

        process_runtime_event(
            &database,
            &RuntimeEvent::ActionItemsDelta(ActionItemsDeltaPayload {
                session_id: "meeting-2".to_string(),
                version: 2,
                updated_at: "2026-04-22T12:01:00Z".to_string(),
                items: vec!["补齐 action-item 前端刷新".to_string()],
                is_final: false,
            }),
        )
        .unwrap();

        let summary = database
            .with_connection(|connection| SummaryRepo::latest_snapshot(connection, "meeting-2"))
            .unwrap()
            .unwrap();

        assert_eq!(summary.abstract_text, "先有纪要主体");
        assert_eq!(
            summary.action_items,
            vec!["补齐 action-item 前端刷新".to_string()]
        );
    }
}
