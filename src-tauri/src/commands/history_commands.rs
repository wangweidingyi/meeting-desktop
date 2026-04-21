use tauri::State;

use crate::app_state::AppState;
use crate::session::models::MeetingRecord;
use crate::storage::meetings_repo::MeetingsRepo;

#[tauri::command]
pub fn list_meeting_history(state: State<'_, AppState>) -> Result<Vec<MeetingRecord>, String> {
    state
        .database
        .with_connection(MeetingsRepo::list_all)
        .map_err(|error| error.to_string())
}
