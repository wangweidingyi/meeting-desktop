use std::sync::{Arc, Mutex};

use super::{CaptureStreamDescriptor, PcmFrameCallback};
use crate::audio::coordinator::CaptureSourceKind;
use crate::audio::runtime::MeetingAudioRuntime;
use crate::transport::audio_transport::AudioTransport;

pub fn build_runtime_sink<T>(
    runtime: Arc<Mutex<Option<MeetingAudioRuntime<T>>>>,
    source: CaptureSourceKind,
    descriptor: CaptureStreamDescriptor,
    target_sample_rate_hz: u32,
    target_channels: u16,
) -> PcmFrameCallback
where
    T: AudioTransport + Send + 'static,
{
    Arc::new(move |started_at_ms, samples| {
        let normalized = match normalize_frame(
            &descriptor,
            &samples,
            target_sample_rate_hz,
            target_channels,
        ) {
            Ok(samples) => samples,
            Err(_) => return,
        };

        if let Ok(mut runtime) = runtime.lock() {
            if let Some(runtime) = runtime.as_mut() {
                let _ = runtime.push_source_samples(source.clone(), started_at_ms, &normalized);
            }
        }
    })
}

pub fn normalize_frame(
    descriptor: &CaptureStreamDescriptor,
    samples: &[i16],
    target_sample_rate_hz: u32,
    target_channels: u16,
) -> Result<Vec<i16>, String> {
    if target_sample_rate_hz == 0 || descriptor.sample_rate_hz == 0 {
        return Err("sample rate must be non-zero".to_string());
    }
    if target_channels != 1 {
        return Err("current runtime only supports mono target audio".to_string());
    }
    if descriptor.channels == 0 {
        return Err("source channels must be non-zero".to_string());
    }
    if descriptor.sample_rate_hz % target_sample_rate_hz != 0 {
        return Err("source sample rate must be an integer multiple of target rate".to_string());
    }

    let mono = downmix_to_mono(samples, descriptor.channels);
    let ratio = descriptor.sample_rate_hz / target_sample_rate_hz;
    if ratio <= 1 {
        return Ok(mono);
    }

    Ok(mono.into_iter().step_by(ratio as usize).collect())
}

fn downmix_to_mono(samples: &[i16], channels: u16) -> Vec<i16> {
    if channels == 1 {
        return samples.to_vec();
    }

    samples
        .chunks(channels as usize)
        .map(|frame| {
            let sum: i32 = frame.iter().map(|sample| i32::from(*sample)).sum();
            (sum / i32::try_from(frame.len()).unwrap_or(1))
                .clamp(i32::from(i16::MIN), i32::from(i16::MAX)) as i16
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::env;
    use std::sync::{Arc, Mutex};
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::{build_runtime_sink, normalize_frame};
    use crate::audio::coordinator::{AudioCoordinatorConfig, CaptureSourceKind};
    use crate::audio::platform::windows::{AudioDeviceDescriptor, CaptureStreamDescriptor};
    use crate::audio::runtime::MeetingAudioRuntime;
    use crate::storage::checkpoint_repo::CheckpointRepo;
    use crate::storage::db::Database;
    use crate::transport::udp_audio::{InMemoryUdpSocket, UdpAudioTransport};

    fn unique_temp_dir(label: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        env::temp_dir().join(format!("meeting-runtime-sink-{label}-{nanos}"))
    }

    #[test]
    fn normalize_frame_downmixes_stereo_and_downsamples_to_16khz() {
        let descriptor = CaptureStreamDescriptor {
            device: AudioDeviceDescriptor {
                id: "loopback".to_string(),
                name: "Loopback".to_string(),
                is_default: true,
            },
            sample_rate_hz: 48_000,
            channels: 2,
        };
        let samples = vec![
            1000, 3000, 2000, 4000, -1000, 1000, -2000, 2000, 500, 1500, 600, 1800,
        ];

        let normalized = normalize_frame(&descriptor, &samples, 16_000, 1).unwrap();

        assert_eq!(normalized, vec![2000, 0]);
    }

    #[test]
    fn runtime_sink_pushes_normalized_frames_into_meeting_runtime() {
        let database = Database::open_in_memory().unwrap();
        let mut runtime = MeetingAudioRuntime::new(
            database.clone(),
            unique_temp_dir("runtime"),
            UdpAudioTransport::new("meeting-1", InMemoryUdpSocket::default()),
            AudioCoordinatorConfig::new("meeting-1"),
        );
        runtime.prepare().unwrap();
        runtime.start_capture().unwrap();

        let shared = Arc::new(Mutex::new(Some(runtime)));
        let sink = build_runtime_sink(
            shared.clone(),
            CaptureSourceKind::SystemLoopback,
            CaptureStreamDescriptor {
                device: AudioDeviceDescriptor {
                    id: "loopback".to_string(),
                    name: "Loopback".to_string(),
                    is_default: true,
                },
                sample_rate_hz: 48_000,
                channels: 2,
            },
            16_000,
            1,
        );

        if let Ok(mut guard) = shared.lock() {
            if let Some(runtime) = guard.as_mut() {
                runtime
                    .push_source_samples(CaptureSourceKind::Microphone, 1_000, &vec![1200; 3_200])
                    .unwrap();
            }
        }

        let stereo_frame = [1000_i16, 3000_i16];
        let source_samples = stereo_frame.repeat(9_600);
        sink(1_000, source_samples);

        let checkpoint = database
            .with_connection(|connection| {
                CheckpointRepo::find_by_meeting_id(connection, "meeting-1")
            })
            .unwrap()
            .unwrap();

        assert_eq!(checkpoint.last_uploaded_mixed_ms, 1_200);
        assert_eq!(checkpoint.last_udp_seq_sent, 0);
    }
}
