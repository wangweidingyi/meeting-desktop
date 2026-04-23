use std::fs;
use std::path::Path;

pub fn pcm16_wave_duration_ms(
    path: &Path,
    sample_rate_hz: u32,
    channels: u16,
) -> Result<u64, String> {
    if sample_rate_hz == 0 || channels == 0 {
        return Err("wav reader requires non-zero sample rate and channels".to_string());
    }

    let bytes = fs::read(path).map_err(|error| error.to_string())?;
    if bytes.len() < 44 {
        return Err("wav file is truncated".to_string());
    }

    let sample_count = (bytes.len() - 44) / 2;
    Ok((sample_count as u64 * 1000) / (u64::from(sample_rate_hz) * u64::from(channels)))
}

pub fn read_pcm16_wave_window(
    path: &Path,
    sample_rate_hz: u32,
    channels: u16,
    start_ms: u64,
    end_ms: Option<u64>,
) -> Result<Vec<i16>, String> {
    if sample_rate_hz == 0 || channels == 0 {
        return Err("wav reader requires non-zero sample rate and channels".to_string());
    }

    let bytes = fs::read(path).map_err(|error| error.to_string())?;
    if bytes.len() < 44 {
        return Err("wav file is truncated".to_string());
    }

    let data = &bytes[44..];
    let bytes_per_sample = 2usize;
    let start_sample = samples_for_duration_ms(start_ms, sample_rate_hz, channels);
    let end_sample = end_ms
        .map(|end_ms| samples_for_duration_ms(end_ms, sample_rate_hz, channels))
        .unwrap_or(data.len() / bytes_per_sample)
        .min(data.len() / bytes_per_sample);

    if end_sample <= start_sample {
        return Ok(Vec::new());
    }

    let start_byte = start_sample * bytes_per_sample;
    let end_byte = end_sample * bytes_per_sample;

    Ok(data[start_byte..end_byte]
        .chunks_exact(2)
        .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]))
        .collect())
}

fn samples_for_duration_ms(duration_ms: u64, sample_rate_hz: u32, channels: u16) -> usize {
    ((duration_ms * u64::from(sample_rate_hz) / 1000) * u64::from(channels)) as usize
}

#[cfg(test)]
mod tests {
    use std::env;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::{pcm16_wave_duration_ms, read_pcm16_wave_window};
    use crate::audio::writer::append_pcm16_wave;

    fn unique_path(label: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        env::temp_dir().join(format!("meeting-audio-reader-{label}-{nanos}.wav"))
    }

    #[test]
    fn wave_reader_returns_duration_and_windowed_samples() {
        let path = unique_path("reader");
        append_pcm16_wave(&path, 16_000, 1, &vec![100; 6_400]).unwrap();

        let duration = pcm16_wave_duration_ms(&path, 16_000, 1).unwrap();
        let window = read_pcm16_wave_window(&path, 16_000, 1, 200, Some(300)).unwrap();

        assert_eq!(duration, 400);
        assert_eq!(window.len(), 1_600);
        assert!(window.iter().all(|sample| *sample == 100));
    }
}
