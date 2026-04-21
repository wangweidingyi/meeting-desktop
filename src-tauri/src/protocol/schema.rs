use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AudioFormat {
    pub encoding: String,
    pub sample_rate: u32,
    pub channels: u8,
}
