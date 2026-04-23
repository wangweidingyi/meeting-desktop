use crate::audio::AudioChunk;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioUploadProgress {
    pub sequence: u64,
    pub last_uploaded_mixed_ms: u64,
}

pub trait AudioTransport {
    fn send_audio_chunk(&self, chunk: &AudioChunk) -> Result<AudioUploadProgress, String>;
}
