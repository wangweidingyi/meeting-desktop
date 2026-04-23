use std::collections::BTreeSet;
use std::path::Path;

use crate::audio::chunker::AudioChunker;
use crate::audio::chunker::AudioChunkerConfig;
use crate::audio::writer::AudioAssetPaths;
use crate::audio::AudioChunk;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum CaptureSourceKind {
    Microphone,
    SystemLoopback,
}

impl CaptureSourceKind {
    pub fn label(&self) -> &'static str {
        match self {
            CaptureSourceKind::Microphone => "microphone",
            CaptureSourceKind::SystemLoopback => "system",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AudioUplinkStrategy {
    MixedDualSource,
    PassthroughSingleSource(CaptureSourceKind),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioCoordinatorConfig {
    pub meeting_id: String,
    pub sample_rate_hz: u32,
    pub channels: u16,
    pub bits_per_sample: u16,
    pub chunk_duration_ms: u32,
    pub preserve_wav: bool,
    pub expected_sources: Vec<CaptureSourceKind>,
    pub uplink_strategy: AudioUplinkStrategy,
}

impl AudioCoordinatorConfig {
    pub fn new(meeting_id: impl Into<String>) -> Self {
        Self {
            meeting_id: meeting_id.into(),
            sample_rate_hz: 16_000,
            channels: 1,
            bits_per_sample: 16,
            chunk_duration_ms: 200,
            preserve_wav: true,
            expected_sources: vec![
                CaptureSourceKind::Microphone,
                CaptureSourceKind::SystemLoopback,
            ],
            uplink_strategy: AudioUplinkStrategy::MixedDualSource,
        }
    }

    pub fn single_source_passthrough(
        meeting_id: impl Into<String>,
        source: CaptureSourceKind,
    ) -> Self {
        Self {
            expected_sources: vec![source.clone()],
            uplink_strategy: AudioUplinkStrategy::PassthroughSingleSource(source),
            ..Self::new(meeting_id)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioSessionPlan {
    pub meeting_id: String,
    pub mixed_stream_key: String,
    pub preserve_wav: bool,
    pub chunker_config: AudioChunkerConfig,
    pub uplink_strategy: AudioUplinkStrategy,
}

#[derive(Debug, Clone)]
pub struct AudioCoordinator {
    config: AudioCoordinatorConfig,
    registered_sources: BTreeSet<CaptureSourceKind>,
    started: bool,
}

impl AudioCoordinator {
    pub fn new(config: AudioCoordinatorConfig) -> Self {
        Self {
            config,
            registered_sources: BTreeSet::new(),
            started: false,
        }
    }

    pub fn register_source(&mut self, source: CaptureSourceKind) {
        self.registered_sources.insert(source);
    }

    pub fn source_count(&self) -> usize {
        self.registered_sources.len()
    }

    pub fn is_started(&self) -> bool {
        self.started
    }

    pub fn missing_sources(&self) -> Vec<CaptureSourceKind> {
        self.config
            .expected_sources
            .iter()
            .cloned()
        .filter(|source| !self.registered_sources.contains(source))
        .collect()
    }

    pub fn expected_sources(&self) -> &[CaptureSourceKind] {
        &self.config.expected_sources
    }

    pub fn start(&mut self) -> Result<AudioSessionPlan, String> {
        let missing_sources = self.missing_sources();
        if !missing_sources.is_empty() {
            let missing = missing_sources
                .into_iter()
                .map(|source| source.label())
                .collect::<Vec<_>>()
                .join(", ");

            return Err(format!(
                "audio coordinator requires both microphone and system audio before start; missing: {missing}"
            ));
        }

        self.started = true;

        Ok(AudioSessionPlan {
            meeting_id: self.config.meeting_id.clone(),
            mixed_stream_key: format!("{}:mixed", self.config.meeting_id),
            preserve_wav: self.config.preserve_wav,
            uplink_strategy: self.config.uplink_strategy.clone(),
            chunker_config: AudioChunkerConfig {
                sample_rate_hz: self.config.sample_rate_hz,
                channels: self.config.channels,
                bits_per_sample: self.config.bits_per_sample,
                chunk_duration_ms: self.config.chunk_duration_ms,
            },
        })
    }

    pub fn build_asset_paths(&self, root_dir: &Path) -> AudioAssetPaths {
        AudioAssetPaths::for_meeting(root_dir, &self.config.meeting_id)
    }

    pub fn build_mixed_chunks(&self, started_at_ms: u64, samples: &[i16]) -> Vec<AudioChunk> {
        let mut chunker = AudioChunker::new(AudioChunkerConfig {
            sample_rate_hz: self.config.sample_rate_hz,
            channels: self.config.channels,
            bits_per_sample: self.config.bits_per_sample,
            chunk_duration_ms: self.config.chunk_duration_ms,
        });

        chunker.chunk_samples(started_at_ms, samples)
    }
}
