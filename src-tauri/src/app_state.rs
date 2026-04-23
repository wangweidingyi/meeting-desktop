use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use tauri::{AppHandle, Manager};

use crate::audio::platform::PlatformCaptureRuntime;
use crate::audio::runtime::MeetingAudioRuntime;
use crate::config::BackendRuntimeConfig;
use crate::events::bus::EventBus;
use crate::session::manager::SessionManager;
use crate::storage::db::Database;
use crate::transport::runtime::{AudioTransportRuntime, SessionTransportRuntime};

pub struct AppState {
    pub database: Database,
    pub events: EventBus,
    pub runtime_config: BackendRuntimeConfig,
    pub audio_root_dir: PathBuf,
    pub session_manager: Arc<Mutex<SessionManager>>,
    pub session_runtime: Arc<Mutex<Option<SessionTransportRuntime>>>,
    pub audio_runtime: Arc<Mutex<Option<MeetingAudioRuntime<AudioTransportRuntime>>>>,
    pub platform_capture_runtime: Arc<Mutex<Option<PlatformCaptureRuntime>>>,
}

impl AppState {
    pub fn new(app_handle: &AppHandle) -> Result<Self, String> {
        let app_data_dir = app_handle
            .path()
            .app_data_dir()
            .map_err(|error| error.to_string())?;

        fs::create_dir_all(&app_data_dir).map_err(|error| error.to_string())?;
        let audio_root_dir = app_data_dir.join("audio");
        fs::create_dir_all(&audio_root_dir).map_err(|error| error.to_string())?;

        let db_path = app_data_dir.join("meeting.sqlite3");
        let database = Database::open(&db_path).map_err(|error| error.to_string())?;
        let runtime_config = BackendRuntimeConfig::from_env()?;

        Ok(Self {
            database,
            events: EventBus::default(),
            runtime_config,
            audio_root_dir,
            session_manager: Arc::new(Mutex::new(SessionManager::default())),
            session_runtime: Arc::new(Mutex::new(None)),
            audio_runtime: Arc::new(Mutex::new(None)),
            platform_capture_runtime: Arc::new(Mutex::new(None)),
        })
    }
}
