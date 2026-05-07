mod app_state;
mod audio;
pub mod backend_sync;
mod commands;
mod config;
mod events;
mod export;
mod protocol;
pub mod session;
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
            let backend_sync = state.backend_sync.clone();
            let app_handle = app.handle().clone();

            thread::spawn(move || {
                while let Ok(event) = event_receiver.recv() {
                    let _ = events::processor::process_runtime_event(backend_sync.as_ref(), &event);
                    let _ = events::processor::emit_runtime_event(&app_handle, &event);
                }
            });

            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::meeting_commands::create_meeting,
            commands::meeting_commands::get_runtime_backend_info,
            commands::meeting_commands::set_backend_auth_token,
            commands::meeting_commands::clear_backend_auth_token,
            commands::meeting_commands::list_recoverable_meetings,
            commands::meeting_commands::start_active_meeting,
            commands::meeting_commands::resume_recoverable_meeting,
            commands::meeting_commands::pause_active_meeting,
            commands::meeting_commands::resume_active_meeting,
            commands::meeting_commands::stop_active_meeting,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
