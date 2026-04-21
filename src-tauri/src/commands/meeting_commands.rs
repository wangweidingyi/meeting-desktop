use tauri::State;

use crate::app_state::AppState;
use crate::session::models::{MeetingRecord, SessionEvent};
use crate::storage::meetings_repo::MeetingsRepo;

#[tauri::command]
pub fn create_meeting(state: State<'_, AppState>, title: String) -> Result<MeetingRecord, String> {
    let mut manager = state
        .session_manager
        .lock()
        .map_err(|error| error.to_string())?;

    let meeting = manager.create_meeting(title);

    state
        .database
        .with_connection(|connection| MeetingsRepo::insert(connection, &meeting))
        .map_err(|error| error.to_string())?;

    Ok(meeting)
}

#[tauri::command]
pub fn list_recoverable_meetings(state: State<'_, AppState>) -> Result<Vec<MeetingRecord>, String> {
    state
        .database
        .with_connection(MeetingsRepo::list_recoverable)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn start_active_meeting(state: State<'_, AppState>) -> Result<MeetingRecord, String> {
    mutate_active_meeting(
        state,
        &[
            SessionEvent::ConnectRequested,
            SessionEvent::ConnectSucceeded,
            SessionEvent::RecordingStarted,
        ],
    )
}

#[tauri::command]
pub fn pause_active_meeting(state: State<'_, AppState>) -> Result<MeetingRecord, String> {
    mutate_active_meeting(state, &[SessionEvent::PauseRequested])
}

#[tauri::command]
pub fn resume_active_meeting(state: State<'_, AppState>) -> Result<MeetingRecord, String> {
    mutate_active_meeting(state, &[SessionEvent::ResumeRequested])
}

#[tauri::command]
pub fn stop_active_meeting(state: State<'_, AppState>) -> Result<MeetingRecord, String> {
    mutate_active_meeting(state, &[SessionEvent::StopRequested])
}

fn mutate_active_meeting(
    state: State<'_, AppState>,
    events: &[SessionEvent],
) -> Result<MeetingRecord, String> {
    let updated_meeting = {
        let mut manager = state
            .session_manager
            .lock()
            .map_err(|error| error.to_string())?;

        let mut latest_meeting = None;

        for event in events {
            latest_meeting = Some(manager.transition_active_meeting(event.clone())?);
        }

        latest_meeting.ok_or_else(|| "no session event supplied".to_string())?
    };

    state
        .database
        .with_connection(|connection| MeetingsRepo::upsert(connection, &updated_meeting))
        .map_err(|error| error.to_string())?;

    Ok(updated_meeting)
}
