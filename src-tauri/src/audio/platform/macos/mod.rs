use std::fmt;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::thread::JoinHandle;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, SampleRate, Stream, StreamConfig};

pub type PcmFrameCallback = Arc<dyn Fn(u64, Vec<i16>) + Send + Sync + 'static>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MacosCaptureStreamDescriptor {
    pub device_name: String,
    pub sample_rate_hz: u32,
    pub channels: u16,
}

pub struct MacosCaptureRuntime {
    descriptor: MacosCaptureStreamDescriptor,
    stop_tx: Mutex<Option<mpsc::Sender<()>>>,
    worker: Mutex<Option<JoinHandle<()>>>,
}

impl MacosCaptureRuntime {
    fn new(
        descriptor: MacosCaptureStreamDescriptor,
        stop_tx: mpsc::Sender<()>,
        worker: JoinHandle<()>,
    ) -> Self {
        Self {
            descriptor,
            stop_tx: Mutex::new(Some(stop_tx)),
            worker: Mutex::new(Some(worker)),
        }
    }

    pub fn stop(&self) {
        if let Ok(mut stop_tx) = self.stop_tx.lock() {
            if let Some(stop_tx) = stop_tx.take() {
                let _ = stop_tx.send(());
            }
        }

        if let Ok(mut worker) = self.worker.lock() {
            if let Some(worker) = worker.take() {
                let _ = worker.join();
            }
        }
    }
}

impl fmt::Debug for MacosCaptureRuntime {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("MacosCaptureRuntime")
            .field("descriptor", &self.descriptor)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone)]
pub struct MacosMicrophoneCapture {
    descriptor: MacosCaptureStreamDescriptor,
    config: StreamConfig,
    sample_format: SampleFormat,
}

impl MacosMicrophoneCapture {
    pub fn default() -> Result<Self, String> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or_else(|| "no default macOS microphone device available".to_string())?;
        let device_name = device.name().map_err(|error| error.to_string())?;
        let (config, sample_format) = preferred_input_config(&device)?;
        let descriptor = MacosCaptureStreamDescriptor {
            device_name,
            sample_rate_hz: config.sample_rate.0,
            channels: config.channels,
        };

        Ok(Self {
            descriptor,
            config,
            sample_format,
        })
    }

    pub fn descriptor(&self) -> &MacosCaptureStreamDescriptor {
        &self.descriptor
    }

    pub fn start_with_sink(&self, sink: PcmFrameCallback) -> Result<MacosCaptureRuntime, String> {
        let descriptor = self.descriptor.clone();
        let config = self.config.clone();
        let sample_format = self.sample_format;
        let (stop_tx, stop_rx) = mpsc::channel();
        let (ready_tx, ready_rx) = mpsc::channel();
        let worker = thread::Builder::new()
            .name("meeting-macos-microphone".to_string())
            .spawn(move || {
                let stream = match build_default_input_stream(
                    descriptor.clone(),
                    config,
                    sample_format,
                    sink,
                ) {
                    Ok(stream) => {
                        let _ = ready_tx.send(Ok(()));
                        stream
                    }
                    Err(error) => {
                        let _ = ready_tx.send(Err(error));
                        return;
                    }
                };

                loop {
                    match stop_rx.recv_timeout(Duration::from_millis(200)) {
                        Ok(()) | Err(mpsc::RecvTimeoutError::Disconnected) => break,
                        Err(mpsc::RecvTimeoutError::Timeout) => continue,
                    }
                }

                drop(stream);
            })
            .map_err(|error| error.to_string())?;

        match ready_rx.recv_timeout(Duration::from_secs(2)) {
            Ok(Ok(())) => Ok(MacosCaptureRuntime::new(
                self.descriptor.clone(),
                stop_tx,
                worker,
            )),
            Ok(Err(error)) => {
                let _ = worker.join();
                Err(error)
            }
            Err(error) => {
                let _ = stop_tx.send(());
                let _ = worker.join();
                Err(error.to_string())
            }
        }
    }
}

fn build_default_input_stream(
    descriptor: MacosCaptureStreamDescriptor,
    config: StreamConfig,
    sample_format: SampleFormat,
    sink: PcmFrameCallback,
) -> Result<Stream, String> {
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or_else(|| "no default macOS microphone device available".to_string())?;
    let error_callback = |error| eprintln!("macOS microphone stream error: {error}");

    let stream = match sample_format {
        SampleFormat::I16 => device.build_input_stream(
            &config,
            {
                let descriptor = descriptor.clone();
                let sink = sink.clone();
                move |data: &[i16], _| forward_input_frame(&descriptor, data, &sink)
            },
            error_callback,
            None,
        ),
        SampleFormat::U16 => device.build_input_stream(
            &config,
            {
                let descriptor = descriptor.clone();
                let sink = sink.clone();
                move |data: &[u16], _| {
                    let samples = data
                        .iter()
                        .map(|sample| (*sample as i32 - i32::from(u16::MAX) / 2) as i16)
                        .collect::<Vec<_>>();
                    forward_input_frame(&descriptor, &samples, &sink);
                }
            },
            error_callback,
            None,
        ),
        SampleFormat::F32 => device.build_input_stream(
            &config,
            {
                let descriptor = descriptor.clone();
                let sink = sink.clone();
                move |data: &[f32], _| {
                    let samples = data
                        .iter()
                        .map(|sample| {
                            let clamped = sample.clamp(-1.0, 1.0);
                            (clamped * f32::from(i16::MAX)).round() as i16
                        })
                        .collect::<Vec<_>>();
                    forward_input_frame(&descriptor, &samples, &sink);
                }
            },
            error_callback,
            None,
        ),
        other => {
            return Err(format!(
                "unsupported macOS microphone sample format: {other:?}"
            ))
        }
    }
    .map_err(|error| error.to_string())?;

    stream.play().map_err(|error| error.to_string())?;
    Ok(stream)
}

fn preferred_input_config(device: &cpal::Device) -> Result<(StreamConfig, SampleFormat), String> {
    let supported_configs = device
        .supported_input_configs()
        .map_err(|error| error.to_string())?
        .collect::<Vec<_>>();

    for supported in &supported_configs {
        if supported.channels() == 1
            && supported.min_sample_rate().0 <= 16_000
            && supported.max_sample_rate().0 >= 16_000
        {
            let config = supported.with_sample_rate(SampleRate(16_000));
            return Ok((config.config(), config.sample_format()));
        }
    }

    let default = device
        .default_input_config()
        .map_err(|error| error.to_string())?;
    Ok((default.config(), default.sample_format()))
}

fn forward_input_frame(
    descriptor: &MacosCaptureStreamDescriptor,
    samples: &[i16],
    sink: &PcmFrameCallback,
) {
    if samples.is_empty() {
        return;
    }

    let normalized = normalize_samples(samples, descriptor.sample_rate_hz, descriptor.channels);
    if normalized.is_empty() {
        return;
    }

    sink(current_unix_ms(), normalized);
}

fn current_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

fn normalize_samples(samples: &[i16], sample_rate_hz: u32, channels: u16) -> Vec<i16> {
    let mono = downmix_to_mono(samples, channels);
    resample_to_target_rate(&mono, sample_rate_hz, 16_000)
}

fn downmix_to_mono(samples: &[i16], channels: u16) -> Vec<i16> {
    if channels <= 1 {
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

fn resample_to_target_rate(samples: &[i16], source_rate_hz: u32, target_rate_hz: u32) -> Vec<i16> {
    if samples.is_empty() || source_rate_hz == 0 || target_rate_hz == 0 {
        return Vec::new();
    }
    if source_rate_hz == target_rate_hz {
        return samples.to_vec();
    }

    let target_len = ((samples.len() as f64) * f64::from(target_rate_hz)
        / f64::from(source_rate_hz))
    .round() as usize;
    let last_index = samples.len().saturating_sub(1);

    (0..target_len)
        .map(|target_index| {
            let source_position =
                (target_index as f64) * f64::from(source_rate_hz) / f64::from(target_rate_hz);
            let source_index = source_position.floor() as usize;
            let next_index = (source_index + 1).min(last_index);
            let fraction = source_position - (source_index as f64);
            let current = f64::from(samples[source_index.min(last_index)]);
            let next = f64::from(samples[next_index]);
            (current + ((next - current) * fraction))
                .round()
                .clamp(f64::from(i16::MIN), f64::from(i16::MAX)) as i16
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{downmix_to_mono, resample_to_target_rate};

    #[test]
    fn normalize_helpers_downmix_and_resample_microphone_frames() {
        let stereo = vec![1000_i16, 3000, 2000, 4000, 3000, 5000, 4000, 6000];
        let mono = downmix_to_mono(&stereo, 2);
        let resampled = resample_to_target_rate(&mono, 48_000, 16_000);

        assert_eq!(mono, vec![2000, 3000, 4000, 5000]);
        assert_eq!(resampled, vec![2000]);
    }
}
