use std::fs::OpenOptions;
use std::io::{Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioAssetPaths {
    pub root_dir: PathBuf,
    pub mic_original_path: PathBuf,
    pub system_original_path: PathBuf,
    pub mixed_uplink_path: PathBuf,
}

impl AudioAssetPaths {
    pub fn for_meeting(root_dir: &Path, meeting_id: &str) -> Self {
        let meeting_dir = root_dir.join(meeting_id);

        Self {
            root_dir: meeting_dir.clone(),
            mic_original_path: meeting_dir.join("mic-original.wav"),
            system_original_path: meeting_dir.join("system-original.wav"),
            mixed_uplink_path: meeting_dir.join("mixed-uplink.wav"),
        }
    }
}

pub fn append_pcm16_wave(
    path: &Path,
    sample_rate_hz: u32,
    channels: u16,
    samples: &[i16],
) -> Result<(), String> {
    if sample_rate_hz == 0 || channels == 0 {
        return Err("wav writer requires non-zero sample rate and channels".to_string());
    }

    let mut file = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .open(path)
        .map_err(|error| error.to_string())?;

    let existing_len = file.metadata().map_err(|error| error.to_string())?.len();
    if existing_len == 0 {
        write_header(&mut file, sample_rate_hz, channels, 0)?;
    } else if existing_len < 44 {
        return Err("wav file is truncated".to_string());
    }

    file.seek(SeekFrom::End(0))
        .map_err(|error| error.to_string())?;
    for sample in samples {
        file.write_all(&sample.to_le_bytes())
            .map_err(|error| error.to_string())?;
    }

    let data_size = file
        .metadata()
        .map_err(|error| error.to_string())?
        .len()
        .saturating_sub(44) as u32;
    rewrite_sizes(&mut file, data_size)?;
    Ok(())
}

fn write_header(
    writer: &mut std::fs::File,
    sample_rate_hz: u32,
    channels: u16,
    data_size: u32,
) -> Result<(), String> {
    let byte_rate = sample_rate_hz
        .checked_mul(u32::from(channels))
        .and_then(|value| value.checked_mul(2))
        .ok_or_else(|| "wav byte rate overflow".to_string())?;
    let block_align = channels
        .checked_mul(2)
        .ok_or_else(|| "wav block align overflow".to_string())?;
    let riff_size = 36_u32
        .checked_add(data_size)
        .ok_or_else(|| "wav riff size overflow".to_string())?;

    writer
        .seek(SeekFrom::Start(0))
        .map_err(|error| error.to_string())?;
    writer
        .write_all(b"RIFF")
        .map_err(|error| error.to_string())?;
    writer
        .write_all(&riff_size.to_le_bytes())
        .map_err(|error| error.to_string())?;
    writer
        .write_all(b"WAVE")
        .map_err(|error| error.to_string())?;
    writer
        .write_all(b"fmt ")
        .map_err(|error| error.to_string())?;
    writer
        .write_all(&16_u32.to_le_bytes())
        .map_err(|error| error.to_string())?;
    writer
        .write_all(&1_u16.to_le_bytes())
        .map_err(|error| error.to_string())?;
    writer
        .write_all(&channels.to_le_bytes())
        .map_err(|error| error.to_string())?;
    writer
        .write_all(&sample_rate_hz.to_le_bytes())
        .map_err(|error| error.to_string())?;
    writer
        .write_all(&byte_rate.to_le_bytes())
        .map_err(|error| error.to_string())?;
    writer
        .write_all(&block_align.to_le_bytes())
        .map_err(|error| error.to_string())?;
    writer
        .write_all(&16_u16.to_le_bytes())
        .map_err(|error| error.to_string())?;
    writer
        .write_all(b"data")
        .map_err(|error| error.to_string())?;
    writer
        .write_all(&data_size.to_le_bytes())
        .map_err(|error| error.to_string())
}

fn rewrite_sizes(writer: &mut std::fs::File, data_size: u32) -> Result<(), String> {
    let riff_size = 36_u32
        .checked_add(data_size)
        .ok_or_else(|| "wav riff size overflow".to_string())?;

    writer
        .seek(SeekFrom::Start(4))
        .map_err(|error| error.to_string())?;
    writer
        .write_all(&riff_size.to_le_bytes())
        .map_err(|error| error.to_string())?;
    writer
        .seek(SeekFrom::Start(40))
        .map_err(|error| error.to_string())?;
    writer
        .write_all(&data_size.to_le_bytes())
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use std::env;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::append_pcm16_wave;

    fn unique_path(label: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        env::temp_dir().join(format!("meeting-audio-{label}-{nanos}.wav"))
    }

    #[test]
    fn append_pcm16_wave_keeps_header_and_data_sizes_in_sync() {
        let path = unique_path("writer");

        append_pcm16_wave(&path, 16_000, 1, &[100, -100]).unwrap();
        append_pcm16_wave(&path, 16_000, 1, &[50, -50]).unwrap();

        let bytes = fs::read(&path).unwrap();
        let riff_size = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
        let data_size = u32::from_le_bytes(bytes[40..44].try_into().unwrap());

        assert_eq!(&bytes[0..4], b"RIFF");
        assert_eq!(&bytes[8..12], b"WAVE");
        assert_eq!(bytes.len(), 52);
        assert_eq!(riff_size, 44);
        assert_eq!(data_size, 8);
    }
}
