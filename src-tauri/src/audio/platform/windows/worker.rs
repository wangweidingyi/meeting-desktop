use std::sync::Arc;
use std::thread;

use super::format::CapturedSampleFormat;
use super::{CaptureStreamDescriptor, PcmFrameCallback, WindowsCaptureHandle};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceRole {
    Microphone,
    Loopback,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WasapiCaptureConfig {
    pub role: DeviceRole,
    pub sample_format: CapturedSampleFormat,
    pub chunk_frames: usize,
}

pub fn start_capture_worker(
    descriptor: CaptureStreamDescriptor,
    sink: PcmFrameCallback,
    config: WasapiCaptureConfig,
) -> Result<WindowsCaptureHandle, String> {
    let active = Arc::new(std::sync::atomic::AtomicBool::new(true));
    let worker_active = active.clone();
    let worker_sink = sink.clone();
    let worker_descriptor = descriptor.clone();
    let worker = thread::Builder::new()
        .name(format!("meeting-capture-{}", worker_descriptor.device.id))
        .spawn(move || {
            if let Err(error) =
                run_capture_loop(worker_descriptor, worker_sink, worker_active, config)
            {
                eprintln!("capture worker exited with error: {error}");
            }
        })
        .map_err(|error| error.to_string())?;

    Ok(WindowsCaptureHandle::with_worker(
        descriptor, sink, active, worker,
    ))
}

#[cfg(target_os = "windows")]
fn run_capture_loop(
    descriptor: CaptureStreamDescriptor,
    sink: PcmFrameCallback,
    active: Arc<std::sync::atomic::AtomicBool>,
    config: WasapiCaptureConfig,
) -> Result<(), String> {
    use std::collections::VecDeque;
    use std::sync::atomic::Ordering;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    use super::format::decode_capture_bytes;
    use wasapi::{deinitialize, initialize_mta, DeviceCollection, DeviceEnumerator, Direction};
    use wasapi::{SampleType, StreamMode, WaveFormat};

    struct ComGuard;
    impl Drop for ComGuard {
        fn drop(&mut self) {
            deinitialize();
        }
    }

    fn current_unix_ms() -> Result<u64, String> {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_millis() as u64)
            .map_err(|error| error.to_string())
    }

    fn default_device_for_role(
        enumerator: &DeviceEnumerator,
        role: DeviceRole,
    ) -> Result<wasapi::Device, String> {
        let direction = match role {
            DeviceRole::Microphone => Direction::Capture,
            DeviceRole::Loopback => Direction::Render,
        };
        enumerator
            .get_default_device(&direction)
            .map_err(|error| error.to_string())
    }

    fn resolve_device(
        enumerator: &DeviceEnumerator,
        collection: &DeviceCollection,
        descriptor: &CaptureStreamDescriptor,
        role: DeviceRole,
    ) -> Result<wasapi::Device, String> {
        let count = collection
            .get_nbr_devices()
            .map_err(|error| error.to_string())?;
        for index in 0..count {
            let device = collection
                .get_device_at_index(index)
                .map_err(|error| error.to_string())?;
            let device_id = device
                .get_id()
                .map_err(|error| error.to_string())?
                .to_string();
            if device_id == descriptor.device.id {
                return Ok(device);
            }
        }

        default_device_for_role(enumerator, role)
    }

    initialize_mta().map_err(|error| format!("{error:?}"))?;
    let _com_guard = ComGuard;

    let enumerator = DeviceEnumerator::new().map_err(|error| error.to_string())?;
    let direction = match config.role {
        DeviceRole::Microphone => Direction::Capture,
        DeviceRole::Loopback => Direction::Render,
    };
    let collection = enumerator
        .get_device_collection(&direction)
        .map_err(|error| error.to_string())?;
    let device = resolve_device(&enumerator, &collection, &descriptor, config.role)?;
    let audio_client = device
        .get_iaudioclient()
        .map_err(|error| error.to_string())?;
    let bytes_per_sample = match config.sample_format {
        CapturedSampleFormat::I16 => 2,
        CapturedSampleFormat::F32 => 4,
    };
    let block_align = usize::from(descriptor.channels) * bytes_per_sample;
    let desired_format = match config.sample_format {
        CapturedSampleFormat::I16 => WaveFormat::new(
            descriptor.sample_rate_hz,
            16,
            &SampleType::Int,
            descriptor.channels,
        ),
        CapturedSampleFormat::F32 => WaveFormat::new(
            descriptor.sample_rate_hz,
            32,
            &SampleType::Float,
            descriptor.channels,
        ),
    };

    let stream_mode = StreamMode::EventsShared {
        autoconvert: true,
        buffer_duration_hns: 200_000,
    };
    audio_client
        .initialize_client(&desired_format, &stream_mode, &Direction::Capture)
        .map_err(|error| error.to_string())?;
    let capture_client = audio_client
        .get_audiocaptureclient()
        .map_err(|error| error.to_string())?;
    let buffer_frame_count = audio_client
        .get_bufferframecount()
        .map_err(|error| error.to_string())?;
    let event = audio_client
        .set_get_eventhandle()
        .map_err(|error| error.to_string())?;
    let queue_bytes_capacity = usize::try_from(buffer_frame_count)
        .map_err(|error| error.to_string())?
        .saturating_mul(block_align)
        .saturating_mul(4);
    let chunk_bytes = config
        .chunk_frames
        .saturating_mul(usize::from(descriptor.channels))
        .saturating_mul(bytes_per_sample);
    let mut raw_bytes = VecDeque::with_capacity(queue_bytes_capacity.max(chunk_bytes));

    audio_client
        .start_stream()
        .map_err(|error| error.to_string())?;

    while active.load(Ordering::SeqCst) {
        capture_client
            .read_from_device_to_deque(
                usize::try_from(buffer_frame_count).unwrap_or_default(),
                &mut raw_bytes,
            )
            .map_err(|error| error.to_string())?;

        while raw_bytes.len() >= chunk_bytes {
            let mut chunk = vec![0_u8; chunk_bytes];
            for byte in &mut chunk {
                *byte = raw_bytes.pop_front().unwrap_or_default();
            }

            let samples = decode_capture_bytes(&chunk, config.sample_format)?;
            sink(current_unix_ms()?, samples);
        }

        if !active.load(Ordering::SeqCst) {
            break;
        }

        match event.wait_for_event(200) {
            Ok(()) => {}
            Err(_) => thread::sleep(Duration::from_millis(10)),
        }
    }

    let tail_len = raw_bytes.len() - (raw_bytes.len() % block_align.max(1));
    if tail_len > 0 {
        let mut tail = vec![0_u8; tail_len];
        for byte in &mut tail {
            *byte = raw_bytes.pop_front().unwrap_or_default();
        }
        let samples = decode_capture_bytes(&tail, config.sample_format)?;
        if !samples.is_empty() {
            sink(current_unix_ms()?, samples);
        }
    }

    audio_client
        .stop_stream()
        .map_err(|error| error.to_string())?;

    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn run_capture_loop(
    _descriptor: CaptureStreamDescriptor,
    _sink: PcmFrameCallback,
    _active: Arc<std::sync::atomic::AtomicBool>,
    _config: WasapiCaptureConfig,
) -> Result<(), String> {
    Ok(())
}
