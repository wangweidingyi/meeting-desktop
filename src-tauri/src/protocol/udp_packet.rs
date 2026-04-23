use crate::audio::AudioChunk;

const MAGIC: &[u8; 4] = b"MTNG";
const VERSION: u8 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioSourceType {
    Mixed = 1,
    Microphone = 2,
    SystemLoopback = 3,
}

impl AudioSourceType {
    fn from_byte(value: u8) -> Result<Self, String> {
        match value {
            1 => Ok(AudioSourceType::Mixed),
            2 => Ok(AudioSourceType::Microphone),
            3 => Ok(AudioSourceType::SystemLoopback),
            _ => Err(format!("unsupported audio source type: {value}")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UdpAudioPacket {
    pub version: u8,
    pub source_type: AudioSourceType,
    pub session_id: String,
    pub sequence: u64,
    pub started_at_ms: u64,
    pub duration_ms: u32,
    pub payload: Vec<u8>,
}

impl UdpAudioPacket {
    pub fn from_chunk(
        session_id: impl Into<String>,
        source_type: AudioSourceType,
        chunk: &AudioChunk,
    ) -> Self {
        Self {
            version: VERSION,
            source_type,
            session_id: session_id.into(),
            sequence: chunk.sequence,
            started_at_ms: chunk.started_at_ms,
            duration_ms: chunk.duration_ms,
            payload: chunk.payload.clone(),
        }
    }

    pub fn encode(&self) -> Result<Vec<u8>, String> {
        let session_id_bytes = self.session_id.as_bytes();
        if session_id_bytes.len() > u16::MAX as usize {
            return Err("session id is too long for udp packet encoding".to_string());
        }

        let mut bytes = Vec::with_capacity(32 + session_id_bytes.len() + self.payload.len());
        bytes.extend_from_slice(MAGIC);
        bytes.push(self.version);
        bytes.push(self.source_type as u8);
        bytes.extend_from_slice(&(session_id_bytes.len() as u16).to_be_bytes());
        bytes.extend_from_slice(session_id_bytes);
        bytes.extend_from_slice(&self.sequence.to_be_bytes());
        bytes.extend_from_slice(&self.started_at_ms.to_be_bytes());
        bytes.extend_from_slice(&self.duration_ms.to_be_bytes());
        bytes.extend_from_slice(&(self.payload.len() as u32).to_be_bytes());
        bytes.extend_from_slice(&self.payload);

        Ok(bytes)
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, String> {
        let mut cursor = 0usize;

        let magic = read_exact(bytes, &mut cursor, MAGIC.len())?;
        if magic != MAGIC {
            return Err("invalid udp audio packet magic".to_string());
        }

        let version = read_u8(bytes, &mut cursor)?;
        let source_type = AudioSourceType::from_byte(read_u8(bytes, &mut cursor)?)?;
        let session_len = read_u16(bytes, &mut cursor)? as usize;
        let session_id = String::from_utf8(read_exact(bytes, &mut cursor, session_len)?.to_vec())
            .map_err(|error| error.to_string())?;
        let sequence = read_u64(bytes, &mut cursor)?;
        let started_at_ms = read_u64(bytes, &mut cursor)?;
        let duration_ms = read_u32(bytes, &mut cursor)?;
        let payload_len = read_u32(bytes, &mut cursor)? as usize;
        let payload = read_exact(bytes, &mut cursor, payload_len)?.to_vec();

        Ok(Self {
            version,
            source_type,
            session_id,
            sequence,
            started_at_ms,
            duration_ms,
            payload,
        })
    }
}

fn read_exact<'a>(bytes: &'a [u8], cursor: &mut usize, len: usize) -> Result<&'a [u8], String> {
    let end = cursor.saturating_add(len);
    let slice = bytes
        .get(*cursor..end)
        .ok_or_else(|| "unexpected end of udp audio packet".to_string())?;
    *cursor = end;
    Ok(slice)
}

fn read_u8(bytes: &[u8], cursor: &mut usize) -> Result<u8, String> {
    Ok(read_exact(bytes, cursor, 1)?[0])
}

fn read_u16(bytes: &[u8], cursor: &mut usize) -> Result<u16, String> {
    let raw = read_exact(bytes, cursor, 2)?;
    Ok(u16::from_be_bytes([raw[0], raw[1]]))
}

fn read_u32(bytes: &[u8], cursor: &mut usize) -> Result<u32, String> {
    let raw = read_exact(bytes, cursor, 4)?;
    Ok(u32::from_be_bytes([raw[0], raw[1], raw[2], raw[3]]))
}

fn read_u64(bytes: &[u8], cursor: &mut usize) -> Result<u64, String> {
    let raw = read_exact(bytes, cursor, 8)?;
    Ok(u64::from_be_bytes([
        raw[0], raw[1], raw[2], raw[3], raw[4], raw[5], raw[6], raw[7],
    ]))
}

#[cfg(test)]
mod tests {
    use super::{AudioSourceType, UdpAudioPacket};

    #[test]
    fn udp_packet_round_trips_mixed_chunk_metadata() {
        let packet = UdpAudioPacket {
            version: 1,
            source_type: AudioSourceType::Mixed,
            session_id: "meeting-1".to_string(),
            sequence: 42,
            started_at_ms: 1_000,
            duration_ms: 200,
            payload: vec![1, 2, 3, 4],
        };

        let encoded = packet.encode().unwrap();
        let decoded = UdpAudioPacket::decode(&encoded).unwrap();

        assert_eq!(decoded.sequence, 42);
        assert_eq!(decoded.source_type, AudioSourceType::Mixed);
        assert_eq!(decoded.started_at_ms, 1_000);
        assert_eq!(decoded.duration_ms, 200);
        assert_eq!(decoded.payload, vec![1, 2, 3, 4]);
    }
}
