use serde::Serialize;
use tauri::State;

use crate::app_state::AppState;
use crate::audio::coordinator::{AudioCoordinatorConfig, CaptureSourceKind};
use crate::audio::platform::PlatformCaptureRuntime;
use crate::audio::runtime::MeetingAudioRuntime;
use crate::config::{BackendRuntimeConfig, MacosSystemAudioMode};
use crate::events::types::{AudioUplinkState, RuntimeEvent, SessionSnapshot};
use crate::session::models::{MeetingRecord, SessionEvent};
use crate::storage::meetings_repo::MeetingsRepo;
use crate::transport::control_transport::ControlTransport;
use crate::transport::runtime::SessionTransportFactory;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeBackendInfo {
    pub control_client_id: String,
    pub current_user_id: Option<String>,
    pub current_user_name: Option<String>,
    pub mqtt_broker_url: Option<String>,
    pub audio_target_addr: String,
    pub admin_api_base_url: String,
    pub startup_stt_provider: Option<String>,
    pub startup_stt_model: Option<String>,
    pub startup_stt_resource_id: Option<String>,
}

#[cfg(target_os = "macos")]
use crate::audio::platform::macos::{
    system_audio::MacosSystemAudioCapture, MacosMicrophoneCapture, MacosPlatformCaptureRuntime,
    PcmFrameCallback,
};
#[cfg(target_os = "windows")]
use crate::audio::platform::windows::device_enumerator::WindowsAudioDeviceEnumerator;
#[cfg(target_os = "windows")]
use crate::audio::platform::windows::loopback_capture::WindowsLoopbackCapture;
#[cfg(target_os = "windows")]
use crate::audio::platform::windows::mic_capture::WindowsMicrophoneCapture;
#[cfg(target_os = "windows")]
use crate::audio::platform::windows::runtime_sink::build_runtime_sink;
#[cfg(target_os = "macos")]
use crate::transport::audio_transport::AudioTransport;

#[tauri::command]
pub fn create_meeting(state: State<'_, AppState>, title: String) -> Result<MeetingRecord, String> {
    let meeting = {
        let mut manager = state
            .session_manager
            .lock()
            .map_err(|error| error.to_string())?;

        let meeting = manager.create_meeting(title);
        meeting
    };

    prepare_runtime_for_meeting(&state, &meeting.id)?;

    state
        .database
        .with_connection(|connection| MeetingsRepo::insert(connection, &meeting))
        .map_err(|error| error.to_string())?;

    publish_session_snapshot(&state, Some(meeting.clone()))?;

    Ok(meeting)
}

#[tauri::command]
pub fn get_runtime_backend_info(state: State<'_, AppState>) -> Result<RuntimeBackendInfo, String> {
    Ok(RuntimeBackendInfo {
        control_client_id: state.runtime_config.client_id.clone(),
        current_user_id: state.runtime_config.current_user_id.clone(),
        current_user_name: state.runtime_config.current_user_name.clone(),
        mqtt_broker_url: state.runtime_config.mqtt_broker.clone(),
        audio_target_addr: state.runtime_config.udp_target_addr(),
        admin_api_base_url: state.runtime_config.admin_api_base_url(),
        startup_stt_provider: state.runtime_config.startup_stt_provider.clone(),
        startup_stt_model: state.runtime_config.startup_stt_model.clone(),
        startup_stt_resource_id: state.runtime_config.startup_stt_resource_id.clone(),
    })
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
    start_current_active_meeting(&state)
}

#[tauri::command]
pub fn resume_recoverable_meeting(
    state: State<'_, AppState>,
    meeting_id: String,
) -> Result<MeetingRecord, String> {
    let meeting = state
        .database
        .with_connection(|connection| MeetingsRepo::find_by_id(connection, &meeting_id))
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "recoverable meeting was not found".to_string())?;

    if !meeting.status.is_recoverable() {
        return Err("meeting is not recoverable".to_string());
    }

    stop_platform_capture_runtime(&state)?;

    {
        let mut manager = state
            .session_manager
            .lock()
            .map_err(|error| error.to_string())?;
        manager.activate_existing_meeting(meeting.clone());
    }

    prepare_runtime_for_meeting(&state, &meeting.id)?;
    start_current_active_meeting(&state)
}

fn start_current_active_meeting(state: &State<'_, AppState>) -> Result<MeetingRecord, String> {
    let meeting_title = {
        let manager = state
            .session_manager
            .lock()
            .map_err(|error| error.to_string())?;

        manager
            .active_meeting()
            .map(|meeting| meeting.title.clone())
            .ok_or_else(|| "no active meeting".to_string())?
    };

    {
        let session_runtime = state
            .session_runtime
            .lock()
            .map_err(|error| error.to_string())?;
        let runtime = session_runtime
            .as_ref()
            .ok_or_else(|| "session transport runtime has not been prepared".to_string())?;

        runtime.control_transport().connect()?;
        runtime.control_transport().open_session(&meeting_title)?;
        runtime.control_transport().start_recording()?;
    }

    {
        stop_platform_capture_runtime(state)?;
    }

    {
        let mut audio_runtime = state
            .audio_runtime
            .lock()
            .map_err(|error| error.to_string())?;

        if let Some(runtime) = audio_runtime.as_mut() {
            if let Err(error) = runtime.start_capture() {
                cleanup_failed_session_start(&state)?;
                return Err(error);
            }

            if let Err(error) = runtime.replay_pending_mixed_audio() {
                cleanup_failed_session_start(&state)?;
                return Err(error);
            }
        }
    }

    if let Err(error) = start_platform_audio_capture(state) {
        cleanup_failed_platform_capture_start(state, &error);
        return Err(error);
    }

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
    {
        let session_runtime = state
            .session_runtime
            .lock()
            .map_err(|error| error.to_string())?;

        if let Some(runtime) = session_runtime.as_ref() {
            runtime.control_transport().pause_recording()?;
        }
    }

    stop_platform_capture_runtime(&state)?;
    publish_audio_uplink_state(&state, AudioUplinkState::Paused)?;
    mutate_active_meeting(&state, &[SessionEvent::PauseRequested])
}

#[tauri::command]
pub fn resume_active_meeting(state: State<'_, AppState>) -> Result<MeetingRecord, String> {
    {
        let session_runtime = state
            .session_runtime
            .lock()
            .map_err(|error| error.to_string())?;

        if let Some(runtime) = session_runtime.as_ref() {
            runtime.control_transport().resume_recording()?;
        }
    }

    if let Err(error) = start_platform_audio_capture(&state) {
        compensate_failed_platform_capture_resume(&state, &error);
        return Err(error);
    }
    publish_audio_uplink_state(&state, AudioUplinkState::WaitingForAudio)?;
    mutate_active_meeting(&state, &[SessionEvent::ResumeRequested])
}

#[tauri::command]
pub fn stop_active_meeting(state: State<'_, AppState>) -> Result<MeetingRecord, String> {
    stop_platform_capture_runtime(&state)?;

    {
        let mut audio_runtime = state
            .audio_runtime
            .lock()
            .map_err(|error| error.to_string())?;

        if let Some(runtime) = audio_runtime.as_ref() {
            runtime.stop()?;
        }

        *audio_runtime = None;
    }

    {
        let session_runtime = state
            .session_runtime
            .lock()
            .map_err(|error| error.to_string())?;

        if let Some(runtime) = session_runtime.as_ref() {
            runtime.control_transport().stop_recording()?;
        }
    }

    mutate_active_meeting(
        &state,
        &[SessionEvent::StopRequested, SessionEvent::FlushCompleted],
    )
}

fn prepare_runtime_for_meeting(
    state: &State<'_, AppState>,
    meeting_id: &str,
) -> Result<(), String> {
    clear_runtime_handles(state)?;

    let transport =
        SessionTransportFactory::prepare(&state.runtime_config, meeting_id, state.events.clone())?;
    let mut audio_runtime = MeetingAudioRuntime::new(
        state.database.clone(),
        state.audio_root_dir.clone(),
        transport.audio_transport().clone(),
        build_audio_coordinator_config(&state.runtime_config, meeting_id),
        state.events.clone(),
        transport.audio_target_addr().to_string(),
    );
    audio_runtime.prepare()?;

    let mut session_runtime = state
        .session_runtime
        .lock()
        .map_err(|error| error.to_string())?;
    *session_runtime = Some(transport);

    let mut active_audio_runtime = state
        .audio_runtime
        .lock()
        .map_err(|error| error.to_string())?;
    *active_audio_runtime = Some(audio_runtime);

    Ok(())
}

fn clear_runtime_handles(state: &State<'_, AppState>) -> Result<(), String> {
    stop_platform_capture_runtime(state)?;

    {
        let mut audio_runtime = state
            .audio_runtime
            .lock()
            .map_err(|error| error.to_string())?;
        if let Some(runtime) = audio_runtime.as_ref() {
            let _ = runtime.stop();
        }
        *audio_runtime = None;
    }

    {
        let mut session_runtime = state
            .session_runtime
            .lock()
            .map_err(|error| error.to_string())?;
        if let Some(runtime) = session_runtime.as_ref() {
            let _ = runtime.control_transport().disconnect();
        }
        *session_runtime = None;
    }

    Ok(())
}

fn stop_platform_capture_runtime(state: &State<'_, AppState>) -> Result<(), String> {
    let mut platform_capture_runtime = state
        .platform_capture_runtime
        .lock()
        .map_err(|error| error.to_string())?;
    if let Some(runtime) = platform_capture_runtime.as_ref() {
        runtime.stop();
    }
    *platform_capture_runtime = None;
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PlatformCaptureFailureContext {
    StartMeeting,
    ResumeMeeting,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ControlFailureCleanup {
    StopRecordingAndDisconnect,
    PauseRecording,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PlatformCaptureFailureCleanupPlan {
    stop_audio_runtime: bool,
    control_cleanup: ControlFailureCleanup,
    publish_uplink_state: Option<AudioUplinkState>,
}

fn build_platform_capture_failure_cleanup_plan(
    context: PlatformCaptureFailureContext,
) -> PlatformCaptureFailureCleanupPlan {
    match context {
        PlatformCaptureFailureContext::StartMeeting => PlatformCaptureFailureCleanupPlan {
            stop_audio_runtime: true,
            control_cleanup: ControlFailureCleanup::StopRecordingAndDisconnect,
            publish_uplink_state: None,
        },
        PlatformCaptureFailureContext::ResumeMeeting => PlatformCaptureFailureCleanupPlan {
            stop_audio_runtime: false,
            control_cleanup: ControlFailureCleanup::PauseRecording,
            publish_uplink_state: Some(AudioUplinkState::Paused),
        },
    }
}

fn cleanup_failed_platform_capture_start(state: &State<'_, AppState>, original_error: &str) {
    let plan =
        build_platform_capture_failure_cleanup_plan(PlatformCaptureFailureContext::StartMeeting);
    if let Err(cleanup_error) = apply_platform_capture_failure_cleanup(state, &plan) {
        let _ = state.events.publish(RuntimeEvent::TransportError {
            message: format!(
                "failed to clean up session after platform capture startup failed: {original_error}; cleanup error: {cleanup_error}"
            ),
        });
    }
}

fn compensate_failed_platform_capture_resume(state: &State<'_, AppState>, original_error: &str) {
    let plan =
        build_platform_capture_failure_cleanup_plan(PlatformCaptureFailureContext::ResumeMeeting);
    if let Err(cleanup_error) = apply_platform_capture_failure_cleanup(state, &plan) {
        let _ = state.events.publish(RuntimeEvent::TransportError {
            message: format!(
                "failed to pause session after platform capture resume failed: {original_error}; cleanup error: {cleanup_error}"
            ),
        });
    }
}

fn apply_platform_capture_failure_cleanup(
    state: &State<'_, AppState>,
    plan: &PlatformCaptureFailureCleanupPlan,
) -> Result<(), String> {
    let mut cleanup_errors = Vec::new();
    let mut control_cleanup_succeeded = true;

    if plan.stop_audio_runtime {
        if let Err(error) = stop_active_audio_runtime(state) {
            cleanup_errors.push(format!("audio runtime stop failed: {error}"));
        }
    }

    match plan.control_cleanup {
        ControlFailureCleanup::StopRecordingAndDisconnect => {
            if let Err(error) = cleanup_failed_session_start(state) {
                control_cleanup_succeeded = false;
                cleanup_errors.push(format!("session cleanup failed: {error}"));
            }
        }
        ControlFailureCleanup::PauseRecording => {
            if let Err(error) = pause_session_recording(state) {
                control_cleanup_succeeded = false;
                cleanup_errors.push(format!("session pause failed: {error}"));
            }
        }
    }

    if control_cleanup_succeeded {
        if let Some(uplink_state) = plan.publish_uplink_state.clone() {
            if let Err(error) = publish_audio_uplink_state(state, uplink_state) {
                cleanup_errors.push(format!("audio uplink state publish failed: {error}"));
            }
        }
    }

    if cleanup_errors.is_empty() {
        Ok(())
    } else {
        Err(cleanup_errors.join("; "))
    }
}

fn stop_active_audio_runtime(state: &State<'_, AppState>) -> Result<(), String> {
    let audio_runtime = state
        .audio_runtime
        .lock()
        .map_err(|error| error.to_string())?;
    if let Some(runtime) = audio_runtime.as_ref() {
        runtime.stop()?;
    }
    Ok(())
}

fn pause_session_recording(state: &State<'_, AppState>) -> Result<(), String> {
    let session_runtime = state
        .session_runtime
        .lock()
        .map_err(|error| error.to_string())?;
    if let Some(runtime) = session_runtime.as_ref() {
        runtime.control_transport().pause_recording()?;
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn start_platform_audio_capture(state: &State<'_, AppState>) -> Result<(), String> {
    let enumerator = WindowsAudioDeviceEnumerator;
    let microphone = enumerator
        .list_microphones()
        .into_iter()
        .find(|device| device.is_default)
        .ok_or_else(|| "no default microphone device available".to_string())?;
    let loopback = enumerator
        .default_loopback_device()
        .ok_or_else(|| "no default loopback device available".to_string())?;

    let microphone_capture = WindowsMicrophoneCapture::new(microphone);
    let loopback_capture = WindowsLoopbackCapture::new(loopback);
    let runtime = state.audio_runtime.clone();

    let microphone_handle = microphone_capture.start_with_sink(build_runtime_sink(
        runtime.clone(),
        CaptureSourceKind::Microphone,
        microphone_capture.descriptor().clone(),
        16_000,
        1,
    ))?;
    let loopback_handle = loopback_capture.start_with_sink(build_runtime_sink(
        runtime,
        CaptureSourceKind::SystemLoopback,
        loopback_capture.descriptor().clone(),
        16_000,
        1,
    ))?;

    let mut platform_capture_runtime = state
        .platform_capture_runtime
        .lock()
        .map_err(|error| error.to_string())?;
    *platform_capture_runtime = Some(PlatformCaptureRuntime::Windows(
        crate::audio::platform::windows::WindowsCaptureRuntime::new(
            microphone_handle,
            loopback_handle,
        ),
    ));

    Ok(())
}

#[cfg(target_os = "macos")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MacosMicrophoneSinkMode {
    MicrophoneOnly,
    Mirror,
}

#[cfg(target_os = "macos")]
#[derive(Debug, Clone, PartialEq, Eq)]
struct MacosCaptureStartupPlan {
    microphone_sink: MacosMicrophoneSinkMode,
    start_system_audio: bool,
}

#[cfg(target_os = "macos")]
fn build_macos_capture_startup_plan(mode: MacosSystemAudioMode) -> MacosCaptureStartupPlan {
    match mode {
        MacosSystemAudioMode::Disabled => MacosCaptureStartupPlan {
            microphone_sink: MacosMicrophoneSinkMode::MicrophoneOnly,
            start_system_audio: false,
        },
        MacosSystemAudioMode::MirrorMicrophone => MacosCaptureStartupPlan {
            microphone_sink: MacosMicrophoneSinkMode::Mirror,
            start_system_audio: false,
        },
        MacosSystemAudioMode::System => MacosCaptureStartupPlan {
            microphone_sink: MacosMicrophoneSinkMode::MicrophoneOnly,
            start_system_audio: true,
        },
    }
}

#[cfg(target_os = "macos")]
fn start_platform_audio_capture(state: &State<'_, AppState>) -> Result<(), String> {
    let microphone_capture = MacosMicrophoneCapture::default()?;
    let plan =
        build_macos_capture_startup_plan(state.runtime_config.macos_system_audio_mode.clone());
    let sink = match plan.microphone_sink {
        MacosMicrophoneSinkMode::MicrophoneOnly => build_macos_source_runtime_sink(
            state.audio_runtime.clone(),
            CaptureSourceKind::Microphone,
        ),
        MacosMicrophoneSinkMode::Mirror => {
            build_macos_mirror_runtime_sink(state.audio_runtime.clone())
        }
    };
    let microphone_runtime = microphone_capture.start_with_sink(sink)?;
    let system_runtime = if plan.start_system_audio {
        let system_capture = match MacosSystemAudioCapture::default() {
            Ok(capture) => capture,
            Err(error) => {
                microphone_runtime.stop();
                return Err(error);
            }
        };
        let system_sink = build_macos_source_runtime_sink(
            state.audio_runtime.clone(),
            CaptureSourceKind::SystemLoopback,
        );
        match system_capture.start_with_sink(system_sink) {
            Ok(runtime) => Some(runtime),
            Err(error) => {
                microphone_runtime.stop();
                return Err(error);
            }
        }
    } else {
        None
    };

    let mut platform_capture_runtime = match state.platform_capture_runtime.lock() {
        Ok(runtime) => runtime,
        Err(error) => {
            microphone_runtime.stop();
            if let Some(system_runtime) = &system_runtime {
                system_runtime.stop();
            }
            return Err(error.to_string());
        }
    };
    *platform_capture_runtime = Some(PlatformCaptureRuntime::Macos(
        MacosPlatformCaptureRuntime::new(microphone_runtime, system_runtime),
    ));

    Ok(())
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn start_platform_audio_capture(_state: &State<'_, AppState>) -> Result<(), String> {
    Ok(())
}

fn build_audio_coordinator_config(
    runtime_config: &BackendRuntimeConfig,
    meeting_id: &str,
) -> AudioCoordinatorConfig {
    #[cfg(target_os = "macos")]
    {
        return build_macos_audio_coordinator_config(runtime_config, meeting_id);
    }

    #[cfg(not(target_os = "macos"))]
    {
        AudioCoordinatorConfig::new(meeting_id.to_string())
    }
}

#[cfg(target_os = "macos")]
fn build_macos_audio_coordinator_config(
    runtime_config: &BackendRuntimeConfig,
    meeting_id: &str,
) -> AudioCoordinatorConfig {
    match runtime_config.macos_system_audio_mode {
        MacosSystemAudioMode::System | MacosSystemAudioMode::MirrorMicrophone => {
            AudioCoordinatorConfig::new(meeting_id.to_string())
        }
        MacosSystemAudioMode::Disabled => AudioCoordinatorConfig::single_source_passthrough(
            meeting_id.to_string(),
            CaptureSourceKind::Microphone,
        ),
    }
}

#[cfg(target_os = "macos")]
fn build_macos_source_runtime_sink<T>(
    runtime: std::sync::Arc<std::sync::Mutex<Option<MeetingAudioRuntime<T>>>>,
    source: CaptureSourceKind,
) -> PcmFrameCallback
where
    T: AudioTransport + Send + 'static,
{
    std::sync::Arc::new(move |started_at_ms, samples| {
        if let Ok(mut runtime) = runtime.lock() {
            if let Some(runtime) = runtime.as_mut() {
                let _ = runtime.push_source_samples(source.clone(), started_at_ms, &samples);
            }
        }
    })
}

#[cfg(target_os = "macos")]
fn build_macos_mirror_runtime_sink<T>(
    runtime: std::sync::Arc<std::sync::Mutex<Option<MeetingAudioRuntime<T>>>>,
) -> PcmFrameCallback
where
    T: AudioTransport + Send + 'static,
{
    std::sync::Arc::new(move |started_at_ms, samples| {
        if let Ok(mut runtime) = runtime.lock() {
            if let Some(runtime) = runtime.as_mut() {
                let _ = runtime.push_source_samples(
                    CaptureSourceKind::Microphone,
                    started_at_ms,
                    &samples,
                );
                let _ = runtime.push_source_samples(
                    CaptureSourceKind::SystemLoopback,
                    started_at_ms,
                    &samples,
                );
            }
        }
    })
}

fn cleanup_failed_session_start(state: &State<'_, AppState>) -> Result<(), String> {
    let session_runtime = state
        .session_runtime
        .lock()
        .map_err(|lock_error| lock_error.to_string())?;
    if let Some(session_runtime) = session_runtime.as_ref() {
        let _ = session_runtime.control_transport().stop_recording();
        let _ = session_runtime.control_transport().disconnect();
    }
    Ok(())
}

fn publish_audio_uplink_state(
    state: &State<'_, AppState>,
    uplink_state: AudioUplinkState,
) -> Result<(), String> {
    let audio_runtime = state
        .audio_runtime
        .lock()
        .map_err(|error| error.to_string())?;
    if let Some(runtime) = audio_runtime.as_ref() {
        runtime.publish_uplink_state(uplink_state)?;
    }
    Ok(())
}

fn mutate_active_meeting(
    state: &State<'_, AppState>,
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

    publish_session_snapshot(&state, Some(updated_meeting.clone()))?;

    Ok(updated_meeting)
}

fn publish_session_snapshot(
    state: &State<'_, AppState>,
    meeting: Option<MeetingRecord>,
) -> Result<(), String> {
    let status = meeting
        .as_ref()
        .map(|record| record.status.clone())
        .unwrap_or_default();

    state
        .events
        .publish(RuntimeEvent::SessionUpdated(SessionSnapshot {
            meeting,
            status,
        }))
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use std::sync::{Arc, Mutex};
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::{
        build_audio_coordinator_config, build_macos_capture_startup_plan,
        build_macos_source_runtime_sink, build_platform_capture_failure_cleanup_plan,
        ControlFailureCleanup, MacosMicrophoneSinkMode, PlatformCaptureFailureContext,
    };
    use crate::audio::coordinator::{
        AudioCoordinatorConfig, AudioUplinkStrategy, CaptureSourceKind,
    };
    use crate::audio::runtime::MeetingAudioRuntime;
    use crate::config::{BackendRuntimeConfig, MacosSystemAudioMode};
    use crate::events::bus::EventBus;
    use crate::events::types::AudioUplinkState;
    use crate::storage::checkpoint_repo::CheckpointRepo;
    use crate::storage::db::Database;
    use crate::transport::udp_audio::{InMemoryUdpSocket, UdpAudioTransport};

    fn unique_temp_dir(label: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("meeting-macos-runtime-sink-{label}-{nanos}"))
    }

    #[test]
    fn macos_source_runtime_sink_pushes_system_loopback_samples() {
        let database = Database::open_in_memory().unwrap();
        let mut runtime = MeetingAudioRuntime::new(
            database.clone(),
            unique_temp_dir("source"),
            UdpAudioTransport::new("meeting-1", InMemoryUdpSocket::default()),
            AudioCoordinatorConfig::new("meeting-1"),
            EventBus::default(),
            "127.0.0.1:6000".to_string(),
        );
        runtime.prepare().unwrap();
        runtime.start_capture().unwrap();

        let shared = Arc::new(Mutex::new(Some(runtime)));
        let sink =
            build_macos_source_runtime_sink(shared.clone(), CaptureSourceKind::SystemLoopback);

        if let Ok(mut guard) = shared.lock() {
            if let Some(runtime) = guard.as_mut() {
                runtime
                    .push_source_samples(CaptureSourceKind::Microphone, 1_000, &vec![1000; 3_200])
                    .unwrap();
            }
        }

        sink(1_000, vec![2000; 3_200]);

        let checkpoint = database
            .with_connection(|connection| {
                CheckpointRepo::find_by_meeting_id(connection, "meeting-1")
            })
            .unwrap()
            .unwrap();

        assert_eq!(checkpoint.last_uploaded_mixed_ms, 1_200);
        assert_eq!(checkpoint.last_udp_seq_sent, 0);
    }

    #[test]
    fn macos_system_mode_starts_microphone_and_system_audio() {
        let plan = build_macos_capture_startup_plan(MacosSystemAudioMode::System);

        assert_eq!(
            plan.microphone_sink,
            MacosMicrophoneSinkMode::MicrophoneOnly
        );
        assert!(plan.start_system_audio);
    }

    #[test]
    fn macos_disabled_mode_starts_only_microphone_passthrough() {
        let plan = build_macos_capture_startup_plan(MacosSystemAudioMode::Disabled);

        assert_eq!(
            plan.microphone_sink,
            MacosMicrophoneSinkMode::MicrophoneOnly
        );
        assert!(!plan.start_system_audio);
    }

    #[test]
    fn macos_mirror_microphone_mode_uses_mirror_sink_without_system_audio() {
        let plan = build_macos_capture_startup_plan(MacosSystemAudioMode::MirrorMicrophone);

        assert_eq!(plan.microphone_sink, MacosMicrophoneSinkMode::Mirror);
        assert!(!plan.start_system_audio);
    }

    #[test]
    fn start_meeting_platform_capture_failure_stops_audio_and_remote_session() {
        let plan = build_platform_capture_failure_cleanup_plan(
            PlatformCaptureFailureContext::StartMeeting,
        );

        assert!(plan.stop_audio_runtime);
        assert_eq!(
            plan.control_cleanup,
            ControlFailureCleanup::StopRecordingAndDisconnect
        );
        assert_eq!(plan.publish_uplink_state, None);
    }

    #[test]
    fn resume_platform_capture_failure_pauses_remote_session_and_uplink() {
        let plan = build_platform_capture_failure_cleanup_plan(
            PlatformCaptureFailureContext::ResumeMeeting,
        );

        assert!(!plan.stop_audio_runtime);
        assert_eq!(plan.control_cleanup, ControlFailureCleanup::PauseRecording);
        assert_eq!(plan.publish_uplink_state, Some(AudioUplinkState::Paused));
    }

    #[test]
    fn macos_mirror_mode_switches_to_dual_source_mixed_uplink() {
        let mut runtime_config = BackendRuntimeConfig::default();
        runtime_config.macos_system_audio_mode = MacosSystemAudioMode::MirrorMicrophone;

        let config = build_audio_coordinator_config(&runtime_config, "meeting-dev");

        assert_eq!(
            config.expected_sources,
            vec![
                CaptureSourceKind::Microphone,
                CaptureSourceKind::SystemLoopback,
            ]
        );
        assert_eq!(config.uplink_strategy, AudioUplinkStrategy::MixedDualSource);
    }

    #[test]
    fn macos_system_mode_switches_to_dual_source_mixed_uplink() {
        let mut runtime_config = BackendRuntimeConfig::default();
        runtime_config.macos_system_audio_mode = MacosSystemAudioMode::System;

        let config = build_audio_coordinator_config(&runtime_config, "meeting-system");

        assert_eq!(
            config.expected_sources,
            vec![
                CaptureSourceKind::Microphone,
                CaptureSourceKind::SystemLoopback,
            ]
        );
        assert_eq!(config.uplink_strategy, AudioUplinkStrategy::MixedDualSource);
    }

    #[test]
    fn macos_disabled_mode_keeps_microphone_passthrough() {
        let mut runtime_config = BackendRuntimeConfig::default();
        runtime_config.macos_system_audio_mode = MacosSystemAudioMode::Disabled;

        let config = build_audio_coordinator_config(&runtime_config, "meeting-mic");

        assert_eq!(config.expected_sources, vec![CaptureSourceKind::Microphone]);
        assert_eq!(
            config.uplink_strategy,
            AudioUplinkStrategy::PassthroughSingleSource(CaptureSourceKind::Microphone)
        );
    }
}
