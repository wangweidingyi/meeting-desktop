use std::fs;

use tauri::{AppHandle, Manager};

use crate::events::bus::EventBus;
use crate::session::manager::SessionManager;
use crate::storage::db::Database;

pub struct AppState {
    pub database: Database,
    pub events: EventBus,
    pub session_manager: std::sync::Mutex<SessionManager>,
}

impl AppState {
    pub fn new(app_handle: &AppHandle) -> Result<Self, String> {
        let app_data_dir = app_handle
            .path()
            .app_data_dir()
            .map_err(|error| error.to_string())?;

        fs::create_dir_all(&app_data_dir).map_err(|error| error.to_string())?;

        let db_path = app_data_dir.join("meeting.sqlite3");
        let database = Database::open(&db_path).map_err(|error| error.to_string())?;

        Ok(Self {
            database,
            events: EventBus::default(),
            session_manager: std::sync::Mutex::new(SessionManager::default()),
        })
    }
}
