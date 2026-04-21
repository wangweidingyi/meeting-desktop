pub trait AudioTransport {
    fn send_audio_chunk(&self, payload: &[u8]) -> Result<(), String>;
}
