use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::audio::buffer::PcmSampleBuffer;
use crate::audio::chunker::AudioChunker;
use crate::audio::coordinator::{
    AudioCoordinator, AudioCoordinatorConfig, AudioUplinkStrategy, CaptureSourceKind,
};
use crate::audio::mixer::mix_aligned_sources_to_mono;
use crate::audio::reader::{pcm16_wave_duration_ms, read_pcm16_wave_window};
use crate::audio::timeline::{align_stream_start_ms, duration_ms_for_samples};
use crate::audio::writer::{append_pcm16_wave, AudioAssetPaths};
use crate::events::bus::EventBus;
use crate::events::types::{AudioUplinkState, RuntimeDiagnosticsPayload, RuntimeEvent};
use crate::session::recovery::{plan_recovery, RecoveryPlan};
use crate::storage::audio_repo::{AudioAssetRecord, AudioRepo};
use crate::storage::checkpoint_repo::{CheckpointRepo, SessionCheckpointRecord};
use crate::storage::db::Database;
use crate::transport::audio_transport::{AudioTransport, AudioUploadProgress};

const DEFAULT_BUFFER_WINDOW_MS: u32 = 30_000;

pub struct MeetingAudioRuntime<T>
where
    T: AudioTransport,
{
    database: Database,
    root_dir: PathBuf,
    transport: T,
    coordinator: AudioCoordinator,
    chunker: Option<AudioChunker>,
    asset_paths: Option<AudioAssetPaths>,
    microphone_queue: SourceQueue,
    system_queue: SourceQueue,
    meeting_id: String,
    sample_rate_hz: u32,
    channels: u16,
    uplink_strategy: AudioUplinkStrategy,
    event_bus: EventBus,
    audio_target_addr: String,
}

impl<T> MeetingAudioRuntime<T>
where
    T: AudioTransport,
{
    pub fn new(
        database: Database,
        root_dir: PathBuf,
        transport: T,
        config: AudioCoordinatorConfig,
        event_bus: EventBus,
        audio_target_addr: String,
    ) -> Self {
        let meeting_id = config.meeting_id.clone();
        let sample_rate_hz = config.sample_rate_hz;
        let channels = config.channels;
        let uplink_strategy = config.uplink_strategy.clone();
        let max_buffer_samples =
            samples_for_duration_ms(DEFAULT_BUFFER_WINDOW_MS, sample_rate_hz, channels);

        Self {
            database,
            root_dir,
            transport,
            coordinator: AudioCoordinator::new(config),
            chunker: None,
            asset_paths: None,
            microphone_queue: SourceQueue::new(max_buffer_samples),
            system_queue: SourceQueue::new(max_buffer_samples),
            meeting_id,
            sample_rate_hz,
            channels,
            uplink_strategy,
            event_bus,
            audio_target_addr,
        }
    }

    pub fn prepare(&mut self) -> Result<(), String> {
        let assets = self.coordinator.build_asset_paths(&self.root_dir);
        fs::create_dir_all(&assets.root_dir).map_err(|error| error.to_string())?;
        append_pcm16_wave(
            &assets.mic_original_path,
            self.sample_rate_hz,
            self.channels,
            &[],
        )?;
        append_pcm16_wave(
            &assets.system_original_path,
            self.sample_rate_hz,
            self.channels,
            &[],
        )?;
        append_pcm16_wave(
            &assets.mixed_uplink_path,
            self.sample_rate_hz,
            self.channels,
            &[],
        )?;

        let audio_assets = AudioAssetRecord {
            meeting_id: self.meeting_id.clone(),
            mic_original_path: Some(assets.mic_original_path.display().to_string()),
            system_original_path: Some(assets.system_original_path.display().to_string()),
            mixed_uplink_path: Some(assets.mixed_uplink_path.display().to_string()),
        };
        self.asset_paths = Some(assets);

        let checkpoint = self
            .database
            .with_connection(|connection| {
                AudioRepo::upsert(connection, &audio_assets)?;
                let checkpoint =
                    match CheckpointRepo::find_by_meeting_id(connection, &self.meeting_id)? {
                        Some(existing) => SessionCheckpointRecord {
                            local_recording_state: "prepared".to_string(),
                            updated_at: current_timestamp_label(),
                            ..existing
                        },
                        None => SessionCheckpointRecord {
                            meeting_id: self.meeting_id.clone(),
                            last_control_seq: 0,
                            last_udp_seq_sent: 0,
                            last_uploaded_mixed_ms: 0,
                            last_transcript_segment_revision: 0,
                            last_summary_version: 0,
                            last_action_item_version: 0,
                            local_recording_state: "prepared".to_string(),
                            recovery_token: None,
                            updated_at: current_timestamp_label(),
                        },
                    };

                CheckpointRepo::upsert(connection, &checkpoint)?;
                Ok(checkpoint)
            })
            .map_err(|error| error.to_string())?;

        self.publish_runtime_diagnostics(
            AudioUplinkState::Idle,
            checkpoint.last_uploaded_mixed_ms,
            sequence_for_diagnostics(&checkpoint),
            None,
        )
    }

    pub fn start_capture(&mut self) -> Result<(), String> {
        for source in self.coordinator.expected_sources().to_vec() {
            self.coordinator.register_source(source);
        }

        let plan = self.coordinator.start()?;
        self.chunker = Some(AudioChunker::with_next_sequence(
            plan.chunker_config,
            self.next_chunk_sequence()?,
        ));
        self.uplink_strategy = plan.uplink_strategy;
        self.persist_recording_state("recording")?;

        let checkpoint = self
            .load_checkpoint()?
            .unwrap_or_else(|| default_checkpoint(&self.meeting_id));
        self.publish_runtime_diagnostics(
            AudioUplinkState::WaitingForAudio,
            checkpoint.last_uploaded_mixed_ms,
            sequence_for_diagnostics(&checkpoint),
            None,
        )
    }

    pub fn replay_pending_mixed_audio(&mut self) -> Result<Option<RecoveryPlan>, String> {
        let checkpoint = self
            .load_checkpoint()?
            .ok_or_else(|| "missing session checkpoint for audio recovery".to_string())?;
        let assets = self
            .asset_paths
            .as_ref()
            .ok_or_else(|| "audio runtime has not been prepared".to_string())?;
        let local_mixed_duration_ms = pcm16_wave_duration_ms(
            &assets.mixed_uplink_path,
            self.sample_rate_hz,
            self.channels,
        )?;
        let Some(plan) = plan_recovery(&checkpoint, local_mixed_duration_ms) else {
            return Ok(None);
        };
        self.publish_runtime_diagnostics(
            AudioUplinkState::Replaying,
            checkpoint.last_uploaded_mixed_ms,
            sequence_for_diagnostics(&checkpoint),
            Some((plan.replay_from_ms, plan.replay_until_ms)),
        )?;
        let samples = read_pcm16_wave_window(
            &assets.mixed_uplink_path,
            self.sample_rate_hz,
            self.channels,
            plan.replay_from_ms,
            Some(plan.replay_until_ms),
        )?;

        if !samples.is_empty() {
            self.ingest_mixed_samples_with_state(
                plan.replay_from_ms,
                &samples,
                AudioUplinkState::Replaying,
                Some((plan.replay_from_ms, plan.replay_until_ms)),
            )?;
        }

        let refreshed_checkpoint = self
            .load_checkpoint()?
            .unwrap_or_else(|| default_checkpoint(&self.meeting_id));
        self.publish_runtime_diagnostics(
            AudioUplinkState::WaitingForAudio,
            refreshed_checkpoint.last_uploaded_mixed_ms,
            sequence_for_diagnostics(&refreshed_checkpoint),
            None,
        )?;

        Ok(Some(plan))
    }

    pub fn push_source_samples(
        &mut self,
        source: CaptureSourceKind,
        started_at_ms: u64,
        samples: &[i16],
    ) -> Result<Vec<AudioUploadProgress>, String> {
        if self.chunker.is_none() {
            return Err("audio runtime has not been started".to_string());
        }

        self.append_source_wave(&source, samples)?;

        match source {
            CaptureSourceKind::Microphone => self.microphone_queue.push_samples(
                started_at_ms,
                samples,
                self.sample_rate_hz,
                self.channels,
            ),
            CaptureSourceKind::SystemLoopback => self.system_queue.push_samples(
                started_at_ms,
                samples,
                self.sample_rate_hz,
                self.channels,
            ),
        }

        if let AudioUplinkStrategy::PassthroughSingleSource(primary_source) = &self.uplink_strategy
        {
            if *primary_source == source {
                return self.ingest_passthrough_samples(started_at_ms, samples);
            }
        }

        self.drain_aligned_mixed_samples()
    }

    pub fn ingest_mixed_samples(
        &mut self,
        started_at_ms: u64,
        samples: &[i16],
    ) -> Result<Vec<AudioUploadProgress>, String> {
        self.ingest_mixed_samples_with_state(
            started_at_ms,
            samples,
            AudioUplinkState::Streaming,
            None,
        )
    }

    pub fn publish_uplink_state(&self, state: AudioUplinkState) -> Result<(), String> {
        let checkpoint = self
            .load_checkpoint()?
            .unwrap_or_else(|| default_checkpoint(&self.meeting_id));
        self.publish_runtime_diagnostics(
            state,
            checkpoint.last_uploaded_mixed_ms,
            sequence_for_diagnostics(&checkpoint),
            None,
        )
    }

    fn ingest_mixed_samples_with_state(
        &mut self,
        started_at_ms: u64,
        samples: &[i16],
        uplink_state: AudioUplinkState,
        replay_window: Option<(u64, u64)>,
    ) -> Result<Vec<AudioUploadProgress>, String> {
        let chunker = self
            .chunker
            .as_mut()
            .ok_or_else(|| "audio runtime has not been started".to_string())?;

        let mut progresses = Vec::new();
        for chunk in chunker.chunk_samples(started_at_ms, samples) {
            let progress = self.transport.send_audio_chunk(&chunk)?;
            let sent_at = current_timestamp_label();
            self.database
                .with_connection(|connection| {
                    CheckpointRepo::record_audio_upload(
                        connection,
                        &self.meeting_id,
                        &progress,
                        &sent_at,
                    )
                })
                .map_err(|error| error.to_string())?;
            self.publish_runtime_diagnostics_with_timestamp(
                uplink_state.clone(),
                progress.last_uploaded_mixed_ms,
                Some(progress.sequence),
                Some(sent_at),
                replay_window,
            )?;
            progresses.push(progress);
        }

        Ok(progresses)
    }

    pub fn stop(&self) -> Result<(), String> {
        self.persist_recording_state("stopped")?;
        self.publish_uplink_state(AudioUplinkState::Stopped)
    }

    fn append_source_wave(
        &self,
        source: &CaptureSourceKind,
        samples: &[i16],
    ) -> Result<(), String> {
        let assets = self
            .asset_paths
            .as_ref()
            .ok_or_else(|| "audio runtime has not been prepared".to_string())?;
        let path = match source {
            CaptureSourceKind::Microphone => &assets.mic_original_path,
            CaptureSourceKind::SystemLoopback => &assets.system_original_path,
        };

        append_pcm16_wave(path, self.sample_rate_hz, self.channels, samples)
    }

    fn append_mixed_wave(&self, samples: &[i16]) -> Result<(), String> {
        let assets = self
            .asset_paths
            .as_ref()
            .ok_or_else(|| "audio runtime has not been prepared".to_string())?;

        append_pcm16_wave(
            &assets.mixed_uplink_path,
            self.sample_rate_hz,
            self.channels,
            samples,
        )
    }

    fn ingest_passthrough_samples(
        &mut self,
        started_at_ms: u64,
        samples: &[i16],
    ) -> Result<Vec<AudioUploadProgress>, String> {
        self.append_mixed_wave(samples)?;
        self.ingest_mixed_samples(started_at_ms, samples)
    }

    fn drain_aligned_mixed_samples(&mut self) -> Result<Vec<AudioUploadProgress>, String> {
        let Some(microphone_started_at_ms) = self.microphone_queue.started_at_ms else {
            return Ok(Vec::new());
        };
        let Some(system_started_at_ms) = self.system_queue.started_at_ms else {
            return Ok(Vec::new());
        };

        let aligned_started_at_ms =
            align_stream_start_ms(microphone_started_at_ms, system_started_at_ms);
        self.microphone_queue
            .align_to(aligned_started_at_ms, self.sample_rate_hz, self.channels);
        self.system_queue
            .align_to(aligned_started_at_ms, self.sample_rate_hz, self.channels);

        let pair_count = self.microphone_queue.len().min(self.system_queue.len());
        if pair_count == 0 {
            return Ok(Vec::new());
        }

        let microphone = self
            .microphone_queue
            .take(pair_count, self.sample_rate_hz, self.channels);
        let system = self
            .system_queue
            .take(pair_count, self.sample_rate_hz, self.channels);
        let mixed = mix_aligned_sources_to_mono(&microphone, &system);

        self.append_mixed_wave(&mixed)?;

        self.ingest_mixed_samples(aligned_started_at_ms, &mixed)
    }

    fn persist_recording_state(&self, local_recording_state: &str) -> Result<(), String> {
        self.database
            .with_connection(|connection| {
                let checkpoint =
                    match CheckpointRepo::find_by_meeting_id(connection, &self.meeting_id)? {
                        Some(existing) => SessionCheckpointRecord {
                            local_recording_state: local_recording_state.to_string(),
                            updated_at: current_timestamp_label(),
                            ..existing
                        },
                        None => SessionCheckpointRecord {
                            meeting_id: self.meeting_id.clone(),
                            last_control_seq: 0,
                            last_udp_seq_sent: 0,
                            last_uploaded_mixed_ms: 0,
                            last_transcript_segment_revision: 0,
                            last_summary_version: 0,
                            last_action_item_version: 0,
                            local_recording_state: local_recording_state.to_string(),
                            recovery_token: None,
                            updated_at: current_timestamp_label(),
                        },
                    };

                CheckpointRepo::upsert(connection, &checkpoint)
            })
            .map_err(|error| error.to_string())
    }

    fn load_checkpoint(&self) -> Result<Option<SessionCheckpointRecord>, String> {
        self.database
            .with_connection(|connection| {
                CheckpointRepo::find_by_meeting_id(connection, &self.meeting_id)
            })
            .map_err(|error| error.to_string())
    }

    fn publish_runtime_diagnostics(
        &self,
        audio_uplink_state: AudioUplinkState,
        last_uploaded_mixed_ms: u64,
        last_chunk_sequence: Option<u64>,
        replay_window: Option<(u64, u64)>,
    ) -> Result<(), String> {
        self.publish_runtime_diagnostics_with_timestamp(
            audio_uplink_state,
            last_uploaded_mixed_ms,
            last_chunk_sequence,
            None,
            replay_window,
        )
    }

    fn publish_runtime_diagnostics_with_timestamp(
        &self,
        audio_uplink_state: AudioUplinkState,
        last_uploaded_mixed_ms: u64,
        last_chunk_sequence: Option<u64>,
        last_chunk_sent_at: Option<String>,
        replay_window: Option<(u64, u64)>,
    ) -> Result<(), String> {
        let (replay_from_ms, replay_until_ms) = replay_window
            .map(|(from, until)| (Some(from), Some(until)))
            .unwrap_or((None, None));

        self.event_bus
            .publish(RuntimeEvent::RuntimeDiagnosticsUpdated(
                RuntimeDiagnosticsPayload {
                    session_id: self.meeting_id.clone(),
                    audio_target_addr: self.audio_target_addr.clone(),
                    audio_uplink_state,
                    last_uploaded_mixed_ms,
                    last_chunk_sequence,
                    last_chunk_sent_at,
                    replay_from_ms,
                    replay_until_ms,
                },
            ))
    }

    fn next_chunk_sequence(&self) -> Result<u64, String> {
        Ok(self
            .load_checkpoint()?
            .map(|checkpoint| {
                if checkpoint.last_uploaded_mixed_ms > 0 {
                    checkpoint.last_udp_seq_sent.saturating_add(1)
                } else {
                    0
                }
            })
            .unwrap_or(0))
    }
}

fn sequence_for_diagnostics(checkpoint: &SessionCheckpointRecord) -> Option<u64> {
    if checkpoint.last_uploaded_mixed_ms == 0 {
        None
    } else {
        Some(checkpoint.last_udp_seq_sent)
    }
}

fn default_checkpoint(meeting_id: &str) -> SessionCheckpointRecord {
    SessionCheckpointRecord {
        meeting_id: meeting_id.to_string(),
        last_control_seq: 0,
        last_udp_seq_sent: 0,
        last_uploaded_mixed_ms: 0,
        last_transcript_segment_revision: 0,
        last_summary_version: 0,
        last_action_item_version: 0,
        local_recording_state: "prepared".to_string(),
        recovery_token: None,
        updated_at: current_timestamp_label(),
    }
}

fn current_timestamp_label() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().to_string())
        .unwrap_or_else(|_| "0".to_string())
}

#[derive(Debug, Clone)]
struct SourceQueue {
    started_at_ms: Option<u64>,
    buffer: PcmSampleBuffer,
}

impl SourceQueue {
    fn new(max_samples: usize) -> Self {
        Self {
            started_at_ms: None,
            buffer: PcmSampleBuffer::new(max_samples),
        }
    }

    fn len(&self) -> usize {
        self.buffer.len()
    }

    fn push_samples(
        &mut self,
        started_at_ms: u64,
        samples: &[i16],
        sample_rate_hz: u32,
        channels: u16,
    ) {
        if samples.is_empty() {
            return;
        }

        if self.started_at_ms.is_none() || self.buffer.is_empty() {
            self.started_at_ms = Some(started_at_ms);
        } else if let Some(expected_started_at_ms) =
            self.next_started_at_ms(sample_rate_hz, channels)
        {
            if started_at_ms > expected_started_at_ms {
                let silence_samples = samples_for_duration_ms(
                    (started_at_ms - expected_started_at_ms) as u32,
                    sample_rate_hz,
                    channels,
                );
                if silence_samples > 0 {
                    let silence = vec![0_i16; silence_samples];
                    let overflow = self.buffer.push(&silence);
                    self.advance_start_by_samples(overflow, sample_rate_hz, channels);
                }
            }
        }

        let overflow = self.buffer.push(samples);
        self.advance_start_by_samples(overflow, sample_rate_hz, channels);
    }

    fn align_to(&mut self, target_started_at_ms: u64, sample_rate_hz: u32, channels: u16) {
        let Some(current_started_at_ms) = self.started_at_ms else {
            return;
        };
        if target_started_at_ms <= current_started_at_ms {
            return;
        }

        let drop_samples = samples_for_duration_ms(
            (target_started_at_ms - current_started_at_ms) as u32,
            sample_rate_hz,
            channels,
        );
        let _ = self.take(drop_samples, sample_rate_hz, channels);
    }

    fn take(&mut self, count: usize, sample_rate_hz: u32, channels: u16) -> Vec<i16> {
        let taken = self.buffer.take(count);
        self.advance_start_by_samples(taken.len(), sample_rate_hz, channels);
        taken
    }

    fn next_started_at_ms(&self, sample_rate_hz: u32, channels: u16) -> Option<u64> {
        self.started_at_ms.map(|started_at_ms| {
            started_at_ms
                + u64::from(duration_ms_for_samples(
                    self.buffer.len(),
                    sample_rate_hz,
                    channels,
                ))
        })
    }

    fn advance_start_by_samples(
        &mut self,
        sample_count: usize,
        sample_rate_hz: u32,
        channels: u16,
    ) {
        if sample_count == 0 {
            return;
        }

        if let Some(started_at_ms) = self.started_at_ms.as_mut() {
            *started_at_ms += u64::from(duration_ms_for_samples(
                sample_count,
                sample_rate_hz,
                channels,
            ));
        }

        if self.buffer.is_empty() {
            self.started_at_ms = None;
        }
    }
}

fn samples_for_duration_ms(duration_ms: u32, sample_rate_hz: u32, channels: u16) -> usize {
    if duration_ms == 0 || sample_rate_hz == 0 || channels == 0 {
        return 0;
    }

    ((u64::from(duration_ms) * u64::from(sample_rate_hz) / 1000) * u64::from(channels)) as usize
}

#[cfg(test)]
mod tests {
    use std::env;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::MeetingAudioRuntime;
    use crate::audio::coordinator::{AudioCoordinatorConfig, CaptureSourceKind};
    use crate::audio::writer::append_pcm16_wave;
    use crate::events::bus::EventBus;
    use crate::events::types::RuntimeEvent;
    use crate::protocol::udp_packet::UdpAudioPacket;
    use crate::storage::audio_repo::AudioRepo;
    use crate::storage::checkpoint_repo::{CheckpointRepo, SessionCheckpointRecord};
    use crate::storage::db::Database;
    use crate::transport::udp_audio::{InMemoryUdpSocket, UdpAudioTransport};

    fn unique_temp_dir(label: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        env::temp_dir().join(format!("meeting-desktop-{label}-{nanos}"))
    }

    #[test]
    fn prepare_persists_audio_asset_paths_and_checkpoint() {
        let database = Database::open_in_memory().unwrap();
        let mut runtime = MeetingAudioRuntime::new(
            database.clone(),
            env::temp_dir(),
            UdpAudioTransport::new("meeting-1", InMemoryUdpSocket::default()),
            AudioCoordinatorConfig::new("meeting-1"),
            EventBus::default(),
            "127.0.0.1:6000".to_string(),
        );

        runtime.prepare().unwrap();

        let assets = database
            .with_connection(|connection| AudioRepo::find_by_meeting_id(connection, "meeting-1"))
            .unwrap()
            .unwrap();
        let checkpoint = database
            .with_connection(|connection| {
                CheckpointRepo::find_by_meeting_id(connection, "meeting-1")
            })
            .unwrap()
            .unwrap();

        assert!(assets
            .mic_original_path
            .unwrap()
            .ends_with("meeting-1/mic-original.wav"));
        assert!(assets
            .system_original_path
            .unwrap()
            .ends_with("meeting-1/system-original.wav"));
        assert!(assets
            .mixed_uplink_path
            .unwrap()
            .ends_with("meeting-1/mixed-uplink.wav"));
        assert_eq!(checkpoint.local_recording_state, "prepared");
        assert_eq!(checkpoint.last_uploaded_mixed_ms, 0);
    }

    #[test]
    fn prepare_preserves_existing_upload_checkpoint_progress() {
        let database = Database::open_in_memory().unwrap();
        database
            .with_connection(|connection| {
                CheckpointRepo::upsert(
                    connection,
                    &SessionCheckpointRecord {
                        meeting_id: "meeting-1".to_string(),
                        last_control_seq: 0,
                        last_udp_seq_sent: 9,
                        last_uploaded_mixed_ms: 2_000,
                        last_transcript_segment_revision: 0,
                        last_summary_version: 0,
                        last_action_item_version: 0,
                        local_recording_state: "error".to_string(),
                        recovery_token: Some("recover-1".to_string()),
                        updated_at: "1000".to_string(),
                    },
                )
            })
            .unwrap();

        let mut runtime = MeetingAudioRuntime::new(
            database.clone(),
            env::temp_dir(),
            UdpAudioTransport::new("meeting-1", InMemoryUdpSocket::default()),
            AudioCoordinatorConfig::new("meeting-1"),
            EventBus::default(),
            "127.0.0.1:6000".to_string(),
        );

        runtime.prepare().unwrap();

        let checkpoint = database
            .with_connection(|connection| {
                CheckpointRepo::find_by_meeting_id(connection, "meeting-1")
            })
            .unwrap()
            .unwrap();

        assert_eq!(checkpoint.last_udp_seq_sent, 9);
        assert_eq!(checkpoint.last_uploaded_mixed_ms, 2_000);
        assert_eq!(checkpoint.recovery_token, Some("recover-1".to_string()));
        assert_eq!(checkpoint.local_recording_state, "prepared");
    }

    #[test]
    fn ingest_mixed_samples_updates_upload_checkpoint() {
        let database = Database::open_in_memory().unwrap();
        let mut runtime = MeetingAudioRuntime::new(
            database.clone(),
            env::temp_dir(),
            UdpAudioTransport::new("meeting-1", InMemoryUdpSocket::default()),
            AudioCoordinatorConfig::new("meeting-1"),
            EventBus::default(),
            "127.0.0.1:6000".to_string(),
        );

        runtime.prepare().unwrap();
        runtime.start_capture().unwrap();
        runtime
            .ingest_mixed_samples(1_000, &vec![320; 6_400])
            .unwrap();

        let checkpoint = database
            .with_connection(|connection| {
                CheckpointRepo::find_by_meeting_id(connection, "meeting-1")
            })
            .unwrap()
            .unwrap();

        assert_eq!(checkpoint.last_udp_seq_sent, 1);
        assert_eq!(checkpoint.last_uploaded_mixed_ms, 1_400);
        assert_eq!(checkpoint.local_recording_state, "recording");
    }

    #[test]
    fn audio_runtime_publishes_runtime_diagnostics_on_upload() {
        let database = Database::open_in_memory().unwrap();
        let event_bus = EventBus::default();
        let mut runtime = MeetingAudioRuntime::new(
            database.clone(),
            env::temp_dir(),
            UdpAudioTransport::new("meeting-1", InMemoryUdpSocket::default()),
            AudioCoordinatorConfig::new("meeting-1"),
            event_bus.clone(),
            "127.0.0.1:6000".to_string(),
        );

        runtime.prepare().unwrap();
        runtime.start_capture().unwrap();
        runtime
            .ingest_mixed_samples(1_000, &vec![320; 3_200])
            .unwrap();

        let events = event_bus.snapshot().unwrap();
        assert!(events.iter().any(|event| matches!(
            event,
            RuntimeEvent::RuntimeDiagnosticsUpdated(payload)
                if payload.session_id == "meeting-1"
                    && payload.audio_target_addr == "127.0.0.1:6000"
                    && payload.audio_uplink_state == crate::events::types::AudioUplinkState::Streaming
                    && payload.last_uploaded_mixed_ms == 1_200
                    && payload.last_chunk_sequence == Some(0)
        )));
    }

    #[test]
    fn push_source_samples_writes_source_and_mixed_wav_then_updates_checkpoint() {
        let database = Database::open_in_memory().unwrap();
        let temp_dir = unique_temp_dir("source-ingress");
        let socket = InMemoryUdpSocket::default();
        let mut runtime = MeetingAudioRuntime::new(
            database.clone(),
            temp_dir,
            UdpAudioTransport::new("meeting-1", socket.clone()),
            AudioCoordinatorConfig::new("meeting-1"),
            EventBus::default(),
            "127.0.0.1:6000".to_string(),
        );

        runtime.prepare().unwrap();
        runtime.start_capture().unwrap();

        let mic_progress = runtime
            .push_source_samples(CaptureSourceKind::Microphone, 1_000, &vec![1000; 3_200])
            .unwrap();
        let mixed_progress = runtime
            .push_source_samples(CaptureSourceKind::SystemLoopback, 1_000, &vec![2000; 3_200])
            .unwrap();

        let assets = database
            .with_connection(|connection| AudioRepo::find_by_meeting_id(connection, "meeting-1"))
            .unwrap()
            .unwrap();
        let checkpoint = database
            .with_connection(|connection| {
                CheckpointRepo::find_by_meeting_id(connection, "meeting-1")
            })
            .unwrap()
            .unwrap();

        assert!(mic_progress.is_empty());
        assert_eq!(mixed_progress.len(), 1);
        assert_eq!(checkpoint.last_udp_seq_sent, 0);
        assert_eq!(checkpoint.last_uploaded_mixed_ms, 1_200);
        assert_eq!(
            fs::metadata(assets.mic_original_path.unwrap())
                .unwrap()
                .len(),
            6_444
        );
        assert_eq!(
            fs::metadata(assets.system_original_path.unwrap())
                .unwrap()
                .len(),
            6_444
        );
        assert_eq!(
            fs::metadata(assets.mixed_uplink_path.unwrap())
                .unwrap()
                .len(),
            6_444
        );
        assert!(socket.take_last_packet().is_some());
    }

    #[test]
    fn push_source_samples_aligns_to_later_stream_start_before_upload() {
        let database = Database::open_in_memory().unwrap();
        let temp_dir = unique_temp_dir("alignment");
        let mut runtime = MeetingAudioRuntime::new(
            database.clone(),
            temp_dir,
            UdpAudioTransport::new("meeting-1", InMemoryUdpSocket::default()),
            AudioCoordinatorConfig::new("meeting-1"),
            EventBus::default(),
            "127.0.0.1:6000".to_string(),
        );

        runtime.prepare().unwrap();
        runtime.start_capture().unwrap();

        runtime
            .push_source_samples(CaptureSourceKind::Microphone, 1_000, &vec![1000; 3_200])
            .unwrap();
        let initial_progress = runtime
            .push_source_samples(CaptureSourceKind::SystemLoopback, 1_100, &vec![2000; 3_200])
            .unwrap();
        let progress = runtime
            .push_source_samples(CaptureSourceKind::Microphone, 1_200, &vec![1000; 1_600])
            .unwrap();

        let checkpoint = database
            .with_connection(|connection| {
                CheckpointRepo::find_by_meeting_id(connection, "meeting-1")
            })
            .unwrap()
            .unwrap();

        assert!(initial_progress.is_empty());
        assert_eq!(progress.len(), 1);
        assert_eq!(progress[0].last_uploaded_mixed_ms, 1_300);
        assert_eq!(checkpoint.last_uploaded_mixed_ms, 1_300);
    }

    #[test]
    fn single_source_passthrough_writes_microphone_and_mixed_wav_then_updates_checkpoint() {
        let database = Database::open_in_memory().unwrap();
        let temp_dir = unique_temp_dir("single-source");
        let socket = InMemoryUdpSocket::default();
        let mut runtime = MeetingAudioRuntime::new(
            database.clone(),
            temp_dir,
            UdpAudioTransport::new("meeting-1", socket.clone()),
            AudioCoordinatorConfig::single_source_passthrough(
                "meeting-1",
                CaptureSourceKind::Microphone,
            ),
            EventBus::default(),
            "127.0.0.1:6000".to_string(),
        );

        runtime.prepare().unwrap();
        runtime.start_capture().unwrap();

        let progress = runtime
            .push_source_samples(CaptureSourceKind::Microphone, 1_000, &vec![1200; 3_200])
            .unwrap();

        let assets = database
            .with_connection(|connection| AudioRepo::find_by_meeting_id(connection, "meeting-1"))
            .unwrap()
            .unwrap();
        let checkpoint = database
            .with_connection(|connection| {
                CheckpointRepo::find_by_meeting_id(connection, "meeting-1")
            })
            .unwrap()
            .unwrap();

        assert_eq!(progress.len(), 1);
        assert_eq!(checkpoint.last_udp_seq_sent, 0);
        assert_eq!(checkpoint.last_uploaded_mixed_ms, 1_200);
        assert_eq!(
            fs::metadata(assets.mic_original_path.unwrap())
                .unwrap()
                .len(),
            6_444
        );
        assert_eq!(
            fs::metadata(assets.system_original_path.unwrap())
                .unwrap()
                .len(),
            44
        );
        assert_eq!(
            fs::metadata(assets.mixed_uplink_path.unwrap())
                .unwrap()
                .len(),
            6_444
        );
        assert!(socket.take_last_packet().is_some());
    }

    #[test]
    fn replay_pending_mixed_audio_replays_from_checkpoint_boundary() {
        let database = Database::open_in_memory().unwrap();
        let temp_dir = unique_temp_dir("replay");
        let socket = InMemoryUdpSocket::default();
        let mut runtime = MeetingAudioRuntime::new(
            database.clone(),
            temp_dir,
            UdpAudioTransport::new("meeting-1", socket.clone()),
            AudioCoordinatorConfig::new("meeting-1"),
            EventBus::default(),
            "127.0.0.1:6000".to_string(),
        );

        runtime.prepare().unwrap();

        let assets = database
            .with_connection(|connection| AudioRepo::find_by_meeting_id(connection, "meeting-1"))
            .unwrap()
            .unwrap();
        append_pcm16_wave(
            std::path::Path::new(assets.mixed_uplink_path.as_deref().unwrap()),
            16_000,
            1,
            &vec![320; 6_400],
        )
        .unwrap();
        database
            .with_connection(|connection| {
                CheckpointRepo::upsert(
                    connection,
                    &SessionCheckpointRecord {
                        meeting_id: "meeting-1".to_string(),
                        last_control_seq: 0,
                        last_udp_seq_sent: 0,
                        last_uploaded_mixed_ms: 200,
                        last_transcript_segment_revision: 0,
                        last_summary_version: 0,
                        last_action_item_version: 0,
                        local_recording_state: "recording".to_string(),
                        recovery_token: Some("replay-1".to_string()),
                        updated_at: "1000".to_string(),
                    },
                )
            })
            .unwrap();

        runtime.start_capture().unwrap();
        let plan = runtime.replay_pending_mixed_audio().unwrap().unwrap();
        let packet = UdpAudioPacket::decode(&socket.take_last_packet().unwrap()).unwrap();
        let checkpoint = database
            .with_connection(|connection| {
                CheckpointRepo::find_by_meeting_id(connection, "meeting-1")
            })
            .unwrap()
            .unwrap();

        assert_eq!(plan.replay_from_ms, 200);
        assert_eq!(plan.replay_until_ms, 400);
        assert_eq!(packet.sequence, 1);
        assert_eq!(packet.started_at_ms, 200);
        assert_eq!(checkpoint.last_udp_seq_sent, 1);
        assert_eq!(checkpoint.last_uploaded_mixed_ms, 400);
    }

    #[test]
    fn replay_pending_mixed_audio_keeps_future_live_chunk_sequence_continuous() {
        let database = Database::open_in_memory().unwrap();
        let temp_dir = unique_temp_dir("replay-sequence");
        let socket = InMemoryUdpSocket::default();
        let mut runtime = MeetingAudioRuntime::new(
            database.clone(),
            temp_dir,
            UdpAudioTransport::new("meeting-1", socket.clone()),
            AudioCoordinatorConfig::new("meeting-1"),
            EventBus::default(),
            "127.0.0.1:6000".to_string(),
        );

        runtime.prepare().unwrap();

        let assets = database
            .with_connection(|connection| AudioRepo::find_by_meeting_id(connection, "meeting-1"))
            .unwrap()
            .unwrap();
        append_pcm16_wave(
            std::path::Path::new(assets.mixed_uplink_path.as_deref().unwrap()),
            16_000,
            1,
            &vec![320; 6_400],
        )
        .unwrap();
        database
            .with_connection(|connection| {
                CheckpointRepo::upsert(
                    connection,
                    &SessionCheckpointRecord {
                        meeting_id: "meeting-1".to_string(),
                        last_control_seq: 0,
                        last_udp_seq_sent: 0,
                        last_uploaded_mixed_ms: 200,
                        last_transcript_segment_revision: 0,
                        last_summary_version: 0,
                        last_action_item_version: 0,
                        local_recording_state: "recording".to_string(),
                        recovery_token: Some("replay-1".to_string()),
                        updated_at: "1000".to_string(),
                    },
                )
            })
            .unwrap();

        runtime.start_capture().unwrap();
        runtime.replay_pending_mixed_audio().unwrap();
        runtime
            .ingest_mixed_samples(400, &vec![640; 3_200])
            .unwrap();

        let packets = socket.packets();

        assert_eq!(packets.len(), 2);
        assert_eq!(UdpAudioPacket::decode(&packets[0]).unwrap().sequence, 1);
        assert_eq!(UdpAudioPacket::decode(&packets[1]).unwrap().sequence, 2);
    }
}
