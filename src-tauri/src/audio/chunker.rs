use crate::audio::timeline::duration_ms_for_samples;
use crate::audio::AudioChunk;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioChunkerConfig {
    pub sample_rate_hz: u32,
    pub channels: u16,
    pub bits_per_sample: u16,
    pub chunk_duration_ms: u32,
}

impl AudioChunkerConfig {
    pub fn pcm16_mono_16khz() -> Self {
        Self {
            sample_rate_hz: 16_000,
            channels: 1,
            bits_per_sample: 16,
            chunk_duration_ms: 200,
        }
    }

    pub fn samples_per_chunk(&self) -> usize {
        (self.sample_rate_hz as usize * self.chunk_duration_ms as usize / 1000)
            * self.channels as usize
    }
}

#[derive(Debug, Clone)]
pub struct AudioChunker {
    config: AudioChunkerConfig,
    next_sequence: u64,
    buffered_started_at_ms: Option<u64>,
    buffered_samples: Vec<i16>,
}

impl AudioChunker {
    pub fn new(config: AudioChunkerConfig) -> Self {
        Self {
            config,
            next_sequence: 0,
            buffered_started_at_ms: None,
            buffered_samples: Vec::new(),
        }
    }

    pub fn with_next_sequence(config: AudioChunkerConfig, next_sequence: u64) -> Self {
        Self {
            config,
            next_sequence,
            buffered_started_at_ms: None,
            buffered_samples: Vec::new(),
        }
    }

    pub fn chunk_samples(&mut self, started_at_ms: u64, samples: &[i16]) -> Vec<AudioChunk> {
        let samples_per_chunk = self.config.samples_per_chunk();
        if samples_per_chunk == 0 || samples.is_empty() {
            return Vec::new();
        }

        if self.buffered_started_at_ms.is_none() {
            self.buffered_started_at_ms = Some(started_at_ms);
        }
        self.buffered_samples.extend_from_slice(samples);

        let mut chunks = Vec::new();
        while self.buffered_samples.len() >= samples_per_chunk {
            let chunk_started_at_ms = self
                .buffered_started_at_ms
                .ok_or("missing chunk start timestamp")
                .unwrap_or(started_at_ms);
            let pcm_samples = self.buffered_samples[..samples_per_chunk].to_vec();
            self.buffered_samples.drain(..samples_per_chunk);

            let duration_ms = duration_ms_for_samples(
                pcm_samples.len(),
                self.config.sample_rate_hz,
                self.config.channels,
            );
            chunks.push(AudioChunk {
                sequence: self.take_sequence(),
                started_at_ms: chunk_started_at_ms,
                duration_ms,
                payload: encode_pcm16le(&pcm_samples),
            });

            self.buffered_started_at_ms =
                Some(chunk_started_at_ms.saturating_add(u64::from(duration_ms)));
        }

        if self.buffered_samples.is_empty() {
            self.buffered_started_at_ms = None;
        }

        chunks
    }

    fn take_sequence(&mut self) -> u64 {
        let sequence = self.next_sequence;
        self.next_sequence += 1;
        sequence
    }
}

fn encode_pcm16le(samples: &[i16]) -> Vec<u8> {
    let mut payload = Vec::with_capacity(samples.len() * 2);
    for sample in samples {
        payload.extend_from_slice(&sample.to_le_bytes());
    }
    payload
}
