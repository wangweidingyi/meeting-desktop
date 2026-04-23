#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapturedSampleFormat {
    I16,
    F32,
}

pub fn decode_capture_bytes(
    bytes: &[u8],
    format: CapturedSampleFormat,
) -> Result<Vec<i16>, String> {
    match format {
        CapturedSampleFormat::I16 => decode_i16_le(bytes),
        CapturedSampleFormat::F32 => decode_f32_le(bytes),
    }
}

fn decode_i16_le(bytes: &[u8]) -> Result<Vec<i16>, String> {
    if bytes.len() % 2 != 0 {
        return Err("pcm16 capture payload must be divisible by 2 bytes".to_string());
    }

    Ok(bytes
        .chunks_exact(2)
        .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]))
        .collect())
}

fn decode_f32_le(bytes: &[u8]) -> Result<Vec<i16>, String> {
    if bytes.len() % 4 != 0 {
        return Err("float32 capture payload must be divisible by 4 bytes".to_string());
    }

    Ok(bytes
        .chunks_exact(4)
        .map(|chunk| {
            let sample = f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
            let clamped = sample.clamp(-1.0, 1.0);
            let scaled = if clamped >= 0.0 {
                clamped * f32::from(i16::MAX)
            } else {
                clamped * f32::from(i16::MAX) + clamped.signum()
            };
            scaled.round().clamp(f32::from(i16::MIN), f32::from(i16::MAX)) as i16
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::{decode_capture_bytes, CapturedSampleFormat};

    #[test]
    fn decode_pcm16_le_bytes_into_samples() {
        let bytes = [0x00_u8, 0x80, 0xFF, 0x7F, 0x34, 0x12];

        let decoded = decode_capture_bytes(&bytes, CapturedSampleFormat::I16).unwrap();

        assert_eq!(decoded, vec![i16::MIN, i16::MAX, 0x1234]);
    }

    #[test]
    fn decode_float32_le_bytes_into_pcm16_samples() {
        let bytes = [0.0_f32, 0.5_f32, -0.5_f32, 1.2_f32]
            .into_iter()
            .flat_map(f32::to_le_bytes)
            .collect::<Vec<_>>();

        let decoded = decode_capture_bytes(&bytes, CapturedSampleFormat::F32).unwrap();

        assert_eq!(decoded[0], 0);
        assert!(decoded[1] > 16_000);
        assert!(decoded[2] < -16_000);
        assert_eq!(decoded[3], i16::MAX);
    }
}
