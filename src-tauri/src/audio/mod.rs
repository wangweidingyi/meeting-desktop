pub mod buffer;
pub mod chunker;
pub mod coordinator;
pub mod mixer;
pub mod platform;
pub mod reader;
pub mod runtime;
pub mod timeline;
pub mod writer;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioChunk {
    pub sequence: u64,
    pub started_at_ms: u64,
    pub duration_ms: u32,
    pub payload: Vec<u8>,
}

#[cfg(test)]
mod tests {
    use super::coordinator::{
        AudioCoordinator, AudioCoordinatorConfig, AudioUplinkStrategy, CaptureSourceKind,
    };

    #[test]
    fn audio_coordinator_requires_mic_and_system_sources() {
        let mut coordinator = AudioCoordinator::new(AudioCoordinatorConfig::new("meeting-1"));
        coordinator.register_source(CaptureSourceKind::Microphone);

        let error = coordinator.start().unwrap_err();

        assert!(error.contains("both microphone and system audio"));
    }

    #[test]
    fn audio_coordinator_starts_after_dual_sources_are_registered() {
        let mut coordinator = AudioCoordinator::new(AudioCoordinatorConfig::new("meeting-1"));

        coordinator.register_source(CaptureSourceKind::Microphone);
        coordinator.register_source(CaptureSourceKind::SystemLoopback);

        let session = coordinator.start().unwrap();

        assert_eq!(coordinator.source_count(), 2);
        assert_eq!(session.mixed_stream_key, "meeting-1:mixed");
        assert!(coordinator.is_started());
    }

    #[test]
    fn audio_coordinator_can_start_in_single_source_passthrough_mode() {
        let mut coordinator = AudioCoordinator::new(
            AudioCoordinatorConfig::single_source_passthrough(
                "meeting-1",
                CaptureSourceKind::Microphone,
            ),
        );

        coordinator.register_source(CaptureSourceKind::Microphone);

        let session = coordinator.start().unwrap();

        assert_eq!(coordinator.source_count(), 1);
        assert_eq!(
            session.uplink_strategy,
            AudioUplinkStrategy::PassthroughSingleSource(CaptureSourceKind::Microphone)
        );
        assert!(coordinator.is_started());
    }

    #[test]
    fn mixer_combines_aligned_sources_into_mono_frame() {
        let mixed = super::mixer::mix_aligned_sources_to_mono(
            &[1000, -1000, 2000, -2000],
            &[2000, 2000, -2000, -2000],
        );

        assert_eq!(mixed, vec![1500, 500, 0, -2000]);
    }

    #[test]
    fn chunker_splits_pcm_samples_into_200ms_payloads() {
        let mut chunker = super::chunker::AudioChunker::new(
            super::chunker::AudioChunkerConfig::pcm16_mono_16khz(),
        );
        let samples = vec![320; 6400];

        let chunks = chunker.chunk_samples(1_000, &samples);

        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].sequence, 0);
        assert_eq!(chunks[0].started_at_ms, 1_000);
        assert_eq!(chunks[0].duration_ms, 200);
        assert_eq!(chunks[0].payload.len(), 6400);
        assert_eq!(chunks[1].sequence, 1);
        assert_eq!(chunks[1].started_at_ms, 1_200);
        assert_eq!(chunks[1].duration_ms, 200);
    }
}
