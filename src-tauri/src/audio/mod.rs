#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioChunk {
    pub sequence: u64,
    pub payload: Vec<u8>,
}
