mod app_state;
mod audio;
mod commands;
mod config;
mod events;
mod export;
mod protocol;
pub mod session;
pub mod storage;
mod transport;

use std::io;
use std::thread;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    use tauri::Manager;

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let state = app_state::AppState::new(app.handle()).map_err(io::Error::other)?;
            let event_receiver = state.events.subscribe().map_err(io::Error::other)?;
            let database = state.database.clone();
            let app_handle = app.handle().clone();

            thread::spawn(move || {
                while let Ok(event) = event_receiver.recv() {
                    let _ = events::processor::process_runtime_event(&database, &event);
                    let _ = events::processor::emit_runtime_event(&app_handle, &event);
                }
            });

            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::meeting_commands::create_meeting,
            commands::meeting_commands::get_runtime_backend_info,
            commands::meeting_commands::list_recoverable_meetings,
            commands::meeting_commands::start_active_meeting,
            commands::meeting_commands::resume_recoverable_meeting,
            commands::meeting_commands::pause_active_meeting,
            commands::meeting_commands::resume_active_meeting,
            commands::meeting_commands::stop_active_meeting,
            commands::history_commands::list_meeting_history,
            commands::history_commands::plan_meeting_recovery,
            commands::history_commands::get_meeting_detail,
            commands::export_commands::export_markdown,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
