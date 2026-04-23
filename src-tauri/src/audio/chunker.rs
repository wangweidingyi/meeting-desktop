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
}

impl AudioChunker {
    pub fn new(config: AudioChunkerConfig) -> Self {
        Self {
            config,
            next_sequence: 0,
        }
    }

    pub fn with_next_sequence(config: AudioChunkerConfig, next_sequence: u64) -> Self {
        Self {
            config,
            next_sequence,
        }
    }

    pub fn chunk_samples(&mut self, started_at_ms: u64, samples: &[i16]) -> Vec<AudioChunk> {
        let samples_per_chunk = self.config.samples_per_chunk();
        if samples_per_chunk == 0 || samples.is_empty() {
            return Vec::new();
        }

        samples
            .chunks(samples_per_chunk)
            .scan(started_at_ms, |chunk_started_at_ms, pcm_samples| {
                let duration_ms = duration_ms_for_samples(
                    pcm_samples.len(),
                    self.config.sample_rate_hz,
                    self.config.channels,
                );
                let chunk = AudioChunk {
                    sequence: self.take_sequence(),
                    started_at_ms: *chunk_started_at_ms,
                    duration_ms,
                    payload: encode_pcm16le(pcm_samples),
                };
                *chunk_started_at_ms += u64::from(duration_ms);
                Some(chunk)
            })
            .collect()
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
