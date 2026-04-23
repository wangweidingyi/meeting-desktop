use tauri::State;

use crate::app_state::AppState;
use crate::export::markdown::{export_meeting_markdown, MeetingMarkdownExport};
use crate::storage::meetings_repo::MeetingsRepo;
use crate::storage::summary_repo::SummaryRepo;
use crate::storage::transcript_repo::TranscriptRepo;

#[tauri::command]
pub fn export_markdown(state: State<'_, AppState>, meeting_id: String) -> Result<String, String> {
    state
        .database
        .with_connection(|connection| {
            let meeting = MeetingsRepo::find_by_id(connection, &meeting_id)?
                .ok_or_else(|| rusqlite::Error::QueryReturnedNoRows)?;
            let transcript_segments = TranscriptRepo::list_by_meeting(connection, &meeting_id)?;
            let summary = SummaryRepo::latest_snapshot(connection, &meeting_id)?;

            Ok(export_meeting_markdown(&MeetingMarkdownExport {
                title: meeting.title,
                started_at: meeting.started_at,
                ended_at: meeting.ended_at,
                transcript_segments,
                summary,
            }))
        })
        .map_err(|error| error.to_string())
}
