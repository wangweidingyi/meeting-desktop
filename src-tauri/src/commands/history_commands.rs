use tauri::State;

use crate::app_state::AppState;
use crate::session::models::MeetingRecord;
use crate::session::recovery::{plan_recovery, RecoveryPlan};
use crate::storage::checkpoint_repo::CheckpointRepo;
use crate::storage::meetings_repo::MeetingsRepo;
use crate::storage::summary_repo::SummaryRepo;
use crate::storage::transcript_repo::{TranscriptRepo, TranscriptSegmentRecord};
use serde::{Deserialize, Serialize};

use crate::storage::summary_repo::SummarySnapshotRecord;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MeetingDetailResponse {
    pub meeting: MeetingRecord,
    pub transcript_segments: Vec<TranscriptSegmentRecord>,
    pub summary: Option<SummarySnapshotRecord>,
    pub action_items: Vec<String>,
}

#[tauri::command]
pub fn list_meeting_history(state: State<'_, AppState>) -> Result<Vec<MeetingRecord>, String> {
    state
        .database
        .with_connection(MeetingsRepo::list_all)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn plan_meeting_recovery(
    state: State<'_, AppState>,
    meeting_id: String,
    local_mixed_duration_ms: u64,
) -> Result<Option<RecoveryPlan>, String> {
    state
        .database
        .with_connection(|connection| {
            let checkpoint = CheckpointRepo::find_by_meeting_id(connection, &meeting_id)?;
            Ok(checkpoint
                .and_then(|checkpoint| plan_recovery(&checkpoint, local_mixed_duration_ms)))
        })
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn get_meeting_detail(
    state: State<'_, AppState>,
    meeting_id: String,
) -> Result<MeetingDetailResponse, String> {
    state
        .database
        .with_connection(|connection| {
            let meeting = MeetingsRepo::find_by_id(connection, &meeting_id)?
                .ok_or_else(|| rusqlite::Error::QueryReturnedNoRows)?;
            let transcript_segments = TranscriptRepo::list_by_meeting(connection, &meeting_id)?;
            let summary = SummaryRepo::latest_snapshot(connection, &meeting_id)?;
            let action_items = SummaryRepo::list_action_items(connection, &meeting_id)?;

            Ok(MeetingDetailResponse {
                meeting,
                transcript_segments,
                summary,
                action_items,
            })
        })
        .map_err(|error| error.to_string())
}
