mod app_state;
mod audio;
mod commands;
mod events;
mod export;
mod protocol;
mod session;
mod storage;
mod transport;

use std::io;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    use tauri::Manager;

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let state = app_state::AppState::new(app.handle()).map_err(io::Error::other)?;
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::meeting_commands::create_meeting,
            commands::meeting_commands::list_recoverable_meetings,
            commands::meeting_commands::start_active_meeting,
            commands::meeting_commands::pause_active_meeting,
            commands::meeting_commands::resume_active_meeting,
            commands::meeting_commands::stop_active_meeting,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
