use std::ffi::{c_char, c_void, CStr};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::ptr;
use std::slice;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::thread::JoinHandle;

use super::PcmFrameCallback;

const TARGET_SAMPLE_RATE_HZ: u32 = 16_000;
const ERROR_BUFFER_LEN: usize = 1024;
const SYSTEM_AUDIO_QUEUE_CAPACITY: usize = 8;

type MeetingSystemAudioCallback = unsafe extern "C" fn(
    user_data: *mut c_void,
    started_at_ms: u64,
    samples: *const f32,
    sample_count: usize,
    sample_rate_hz: u32,
    channels: u16,
);

extern "C" {
    fn meeting_system_audio_start(
        callback: Option<MeetingSystemAudioCallback>,
        user_data: *mut c_void,
        out_handle: *mut *mut c_void,
        error_buffer: *mut c_char,
        error_buffer_len: usize,
    ) -> bool;

    fn meeting_system_audio_stop(handle: *mut c_void);
}

pub struct MacosSystemAudioCapture;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MacosSystemAudioDescriptor {
    pub sample_rate_hz: u32,
    pub channels: u16,
}

pub struct MacosSystemCaptureRuntime {
    inner: Arc<MacosSystemCaptureHandle>,
}

struct MacosSystemCaptureHandle {
    callback_state: Arc<BridgeCallbackState>,
    native_handle: Mutex<Option<*mut c_void>>,
    native_callback_state: Mutex<Option<*const BridgeCallbackState>>,
    worker: Mutex<Option<JoinHandle<()>>>,
}

struct BridgeCallbackState {
    active: AtomicBool,
    frame_tx: Mutex<Option<mpsc::SyncSender<RawSystemAudioFrame>>>,
}

struct RawSystemAudioFrame {
    started_at_ms: u64,
    samples: Vec<f32>,
    sample_rate_hz: u32,
    channels: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EnqueueResult {
    Queued,
    Dropped,
    Closed,
}

impl EnqueueResult {
    #[cfg(test)]
    fn is_queued(self) -> bool {
        matches!(self, Self::Queued)
    }

    #[cfg(test)]
    fn is_dropped(self) -> bool {
        matches!(self, Self::Dropped)
    }
}

struct StreamPcmConverter {
    target_sample_rate_hz: u32,
    resampler: Option<StreamResampler>,
}

struct StreamResampler {
    source_rate_hz: u32,
    target_rate_hz: u32,
    source_position: f64,
    pending: Vec<f32>,
}

unsafe impl Send for MacosSystemCaptureHandle {}
unsafe impl Sync for MacosSystemCaptureHandle {}

impl MacosSystemAudioCapture {
    pub fn default() -> Result<Self, String> {
        Ok(Self)
    }

    pub fn start_with_sink(
        &self,
        sink: PcmFrameCallback,
    ) -> Result<MacosSystemCaptureRuntime, String> {
        start_native_system_audio_capture(sink)
    }
}

impl MacosSystemCaptureRuntime {
    pub fn stop(&self) {
        self.inner.stop();
    }

    #[cfg(test)]
    fn from_test_sink(sink: PcmFrameCallback) -> Self {
        let (callback_state, worker) = build_callback_worker(sink).unwrap();
        Self {
            inner: Arc::new(MacosSystemCaptureHandle {
                callback_state,
                native_handle: Mutex::new(None),
                native_callback_state: Mutex::new(None),
                worker: Mutex::new(Some(worker)),
            }),
        }
    }
}

impl Drop for MacosSystemCaptureRuntime {
    fn drop(&mut self) {
        self.stop();
    }
}

impl MacosSystemCaptureHandle {
    fn stop(&self) {
        self.callback_state.active.store(false, Ordering::SeqCst);

        if let Ok(mut native_handle) = self.native_handle.lock() {
            if let Some(native_handle) = native_handle.take() {
                unsafe { meeting_system_audio_stop(native_handle) };
            }
        }

        if let Ok(mut callback_state) = self.native_callback_state.lock() {
            if let Some(callback_state) = callback_state.take() {
                unsafe {
                    drop(Arc::from_raw(callback_state));
                }
            }
        }

        self.callback_state.close();

        if let Ok(mut worker) = self.worker.lock() {
            if let Some(worker) = worker.take() {
                let _ = worker.join();
            }
        }
    }
}

impl Drop for MacosSystemCaptureHandle {
    fn drop(&mut self) {
        self.stop();
    }
}

pub fn convert_f32_interleaved_to_pcm16_mono(
    descriptor: &MacosSystemAudioDescriptor,
    samples: &[f32],
    target_sample_rate_hz: u32,
) -> Result<Vec<i16>, String> {
    if descriptor.sample_rate_hz == 0 || target_sample_rate_hz == 0 {
        return Err("sample rate must be non-zero".to_string());
    }
    if descriptor.channels == 0 {
        return Err("source channels must be non-zero".to_string());
    }
    let mono = downmix_f32_interleaved_to_mono(descriptor, samples);
    let resampled =
        resample_to_target_rate(&mono, descriptor.sample_rate_hz, target_sample_rate_hz);

    Ok(resampled.into_iter().map(f32_to_pcm16).collect())
}

impl StreamPcmConverter {
    fn new(target_sample_rate_hz: u32) -> Self {
        Self {
            target_sample_rate_hz,
            resampler: None,
        }
    }

    fn convert(
        &mut self,
        descriptor: &MacosSystemAudioDescriptor,
        samples: &[f32],
    ) -> Result<Vec<i16>, String> {
        if descriptor.sample_rate_hz == 0 || self.target_sample_rate_hz == 0 {
            return Err("sample rate must be non-zero".to_string());
        }
        if descriptor.channels == 0 {
            return Err("source channels must be non-zero".to_string());
        }

        let mono = downmix_f32_interleaved_to_mono(descriptor, samples);
        let resampler = self.resampler.get_or_insert_with(|| {
            StreamResampler::new(descriptor.sample_rate_hz, self.target_sample_rate_hz)
        });
        if resampler.source_rate_hz != descriptor.sample_rate_hz
            || resampler.target_rate_hz != self.target_sample_rate_hz
        {
            *resampler =
                StreamResampler::new(descriptor.sample_rate_hz, self.target_sample_rate_hz);
        }

        Ok(resampler
            .process(&mono)
            .into_iter()
            .map(f32_to_pcm16)
            .collect())
    }
}

impl StreamResampler {
    fn new(source_rate_hz: u32, target_rate_hz: u32) -> Self {
        Self {
            source_rate_hz,
            target_rate_hz,
            source_position: 0.0,
            pending: Vec::new(),
        }
    }

    fn process(&mut self, samples: &[f32]) -> Vec<f32> {
        if samples.is_empty() || self.source_rate_hz == 0 || self.target_rate_hz == 0 {
            return Vec::new();
        }
        if self.source_rate_hz == self.target_rate_hz {
            return samples.to_vec();
        }

        self.pending.extend_from_slice(samples);
        let source_step = f64::from(self.source_rate_hz) / f64::from(self.target_rate_hz);
        let mut resampled = Vec::new();

        while self.source_position + 1.0 < self.pending.len() as f64 {
            let source_index = self.source_position.floor() as usize;
            let next_index = (source_index + 1).min(self.pending.len().saturating_sub(1));
            let fraction = self.source_position - source_index as f64;
            let current = self.pending[source_index] as f64;
            let next = self.pending[next_index] as f64;

            resampled.push((current + ((next - current) * fraction)) as f32);
            self.source_position += source_step;
        }

        let consumed = self.source_position.floor() as usize;
        if consumed > 0 {
            let drain_count = consumed.min(self.pending.len());
            self.pending.drain(..drain_count);
            self.source_position -= drain_count as f64;
        }

        resampled
    }
}

fn f32_to_pcm16(sample: f32) -> i16 {
    let clamped = sample.clamp(-1.0, 1.0);
    (clamped * f32::from(i16::MAX)).round() as i16
}

fn downmix_f32_interleaved_to_mono(
    descriptor: &MacosSystemAudioDescriptor,
    samples: &[f32],
) -> Vec<f32> {
    if descriptor.channels == 1 {
        return samples.to_vec();
    }

    samples
        .chunks(descriptor.channels as usize)
        .map(|frame| frame.iter().sum::<f32>() / frame.len() as f32)
        .collect()
}

fn resample_to_target_rate(samples: &[f32], source_rate_hz: u32, target_rate_hz: u32) -> Vec<f32> {
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
            let current = samples[source_index.min(last_index)] as f64;
            let next = samples[next_index] as f64;
            (current + ((next - current) * fraction)) as f32
        })
        .collect()
}

fn start_native_system_audio_capture(
    sink: PcmFrameCallback,
) -> Result<MacosSystemCaptureRuntime, String> {
    let (callback_state, worker) = build_callback_worker(sink)?;
    let native_callback_state = Arc::into_raw(callback_state.clone());
    let mut native_handle = ptr::null_mut();
    let mut error_buffer = [0_i8; ERROR_BUFFER_LEN];

    let started = unsafe {
        meeting_system_audio_start(
            Some(system_audio_callback),
            native_callback_state as *mut c_void,
            &mut native_handle,
            error_buffer.as_mut_ptr(),
            error_buffer.len(),
        )
    };

    if !started {
        unsafe {
            drop(Arc::from_raw(native_callback_state));
        }
        callback_state.active.store(false, Ordering::SeqCst);
        callback_state.close();
        let _ = worker.join();
        return Err(format_start_error(&read_error_buffer(&error_buffer)));
    }

    Ok(MacosSystemCaptureRuntime {
        inner: Arc::new(MacosSystemCaptureHandle {
            callback_state,
            native_handle: Mutex::new(Some(native_handle)),
            native_callback_state: Mutex::new(Some(native_callback_state)),
            worker: Mutex::new(Some(worker)),
        }),
    })
}

fn build_callback_worker(
    sink: PcmFrameCallback,
) -> Result<(Arc<BridgeCallbackState>, JoinHandle<()>), String> {
    let (frame_tx, frame_rx) = mpsc::sync_channel(SYSTEM_AUDIO_QUEUE_CAPACITY);
    let callback_state = Arc::new(BridgeCallbackState {
        active: AtomicBool::new(true),
        frame_tx: Mutex::new(Some(frame_tx)),
    });
    let worker_state = callback_state.clone();
    let worker = thread::Builder::new()
        .name("meeting-macos-system-audio".to_string())
        .spawn(move || run_system_audio_worker(worker_state, frame_rx, sink))
        .map_err(|error| error.to_string())?;

    Ok((callback_state, worker))
}

fn run_system_audio_worker(
    callback_state: Arc<BridgeCallbackState>,
    frame_rx: mpsc::Receiver<RawSystemAudioFrame>,
    sink: PcmFrameCallback,
) {
    let mut converter = StreamPcmConverter::new(TARGET_SAMPLE_RATE_HZ);

    while let Ok(frame) = frame_rx.recv() {
        if !callback_state.active.load(Ordering::SeqCst) {
            continue;
        }

        let result = catch_unwind(AssertUnwindSafe(|| {
            forward_system_audio_samples(&callback_state, &sink, &mut converter, frame);
        }));

        if result.is_err() {
            callback_state.active.store(false, Ordering::SeqCst);
            callback_state.close();
            break;
        }
    }
}

unsafe extern "C" fn system_audio_callback(
    user_data: *mut c_void,
    started_at_ms: u64,
    samples: *const f32,
    sample_count: usize,
    sample_rate_hz: u32,
    channels: u16,
) {
    if user_data.is_null() || samples.is_null() || sample_count == 0 {
        return;
    }

    let callback_state = &*(user_data as *const BridgeCallbackState);
    enqueue_system_audio_samples(
        callback_state,
        started_at_ms,
        slice::from_raw_parts(samples, sample_count),
        sample_rate_hz,
        channels,
    );
}

fn enqueue_system_audio_samples(
    callback_state: &BridgeCallbackState,
    started_at_ms: u64,
    samples: &[f32],
    sample_rate_hz: u32,
    channels: u16,
) -> EnqueueResult {
    if !callback_state.active.load(Ordering::SeqCst) {
        return EnqueueResult::Closed;
    }

    let frame = RawSystemAudioFrame {
        started_at_ms,
        samples: samples.to_vec(),
        sample_rate_hz,
        channels,
    };

    let send_result = callback_state
        .frame_tx
        .lock()
        .ok()
        .and_then(|frame_tx| frame_tx.as_ref().map(|frame_tx| frame_tx.try_send(frame)));

    match send_result {
        Some(Ok(())) => EnqueueResult::Queued,
        Some(Err(mpsc::TrySendError::Full(_))) => EnqueueResult::Dropped,
        Some(Err(mpsc::TrySendError::Disconnected(_))) | None => {
            callback_state.active.store(false, Ordering::SeqCst);
            EnqueueResult::Closed
        }
    }
}

fn forward_system_audio_samples(
    callback_state: &BridgeCallbackState,
    sink: &PcmFrameCallback,
    converter: &mut StreamPcmConverter,
    frame: RawSystemAudioFrame,
) {
    if !callback_state.active.load(Ordering::SeqCst) {
        return;
    }

    let descriptor = MacosSystemAudioDescriptor {
        sample_rate_hz: frame.sample_rate_hz,
        channels: frame.channels,
    };
    let pcm = match converter.convert(&descriptor, &frame.samples) {
        Ok(pcm) if !pcm.is_empty() => pcm,
        _ => return,
    };

    if callback_state.active.load(Ordering::SeqCst) {
        (sink)(frame.started_at_ms, pcm);
    }
}

impl BridgeCallbackState {
    fn close(&self) {
        if let Ok(mut frame_tx) = self.frame_tx.lock() {
            frame_tx.take();
        }
    }
}

fn read_error_buffer(error_buffer: &[c_char]) -> String {
    let message = unsafe { CStr::from_ptr(error_buffer.as_ptr()) }
        .to_string_lossy()
        .trim()
        .to_string();

    if message.is_empty() {
        "failed to start macOS system audio capture".to_string()
    } else {
        message
    }
}

fn format_start_error(message: &str) -> String {
    if message.contains("requires ScreenCaptureKit audio capture") {
        return "macOS system audio capture requires a supported macOS version with ScreenCaptureKit audio capture.".to_string();
    }

    if looks_like_screen_recording_permission_denied(message) {
        return format!(
            "macOS denied Screen Recording permission required for system audio capture. Grant Screen Recording permission and retry. Native error: {message}"
        );
    }

    if looks_like_permission_denied(message) {
        return format!(
            "macOS denied system audio capture permission. Grant recording permission and retry. Native error: {message}"
        );
    }

    format!("failed to start macOS system audio capture: {message}")
}

fn looks_like_screen_recording_permission_denied(message: &str) -> bool {
    let normalized = message.to_ascii_lowercase();
    normalized.contains("screen recording permission denied")
        || normalized.contains("screen recording permission")
        || normalized.contains("not authorized to capture screen")
        || normalized.contains("user declined")
}

fn looks_like_permission_denied(message: &str) -> bool {
    let normalized = message.to_ascii_lowercase();
    normalized.contains("permission")
        || normalized.contains("not permitted")
        || normalized.contains("denied")
        || message.contains("OSStatus -54")
        || message.contains("('perm')")
}

#[cfg(test)]
fn invoke_system_audio_callback_for_test(
    runtime: &MacosSystemCaptureRuntime,
    started_at_ms: u64,
    samples: &[f32],
    sample_rate_hz: u32,
    channels: u16,
) {
    enqueue_system_audio_samples(
        &runtime.inner.callback_state,
        started_at_ms,
        samples,
        sample_rate_hz,
        channels,
    );
}

#[cfg(test)]
fn build_callback_state_for_test(
    capacity: usize,
) -> (
    Arc<BridgeCallbackState>,
    mpsc::Receiver<RawSystemAudioFrame>,
) {
    let (frame_tx, frame_rx) = mpsc::sync_channel(capacity);
    (
        Arc::new(BridgeCallbackState {
            active: AtomicBool::new(true),
            frame_tx: Mutex::new(Some(frame_tx)),
        }),
        frame_rx,
    )
}

#[cfg(test)]
fn enqueue_system_audio_samples_for_state_test(
    callback_state: &BridgeCallbackState,
    started_at_ms: u64,
    samples: &[f32],
    sample_rate_hz: u32,
    channels: u16,
) -> EnqueueResult {
    enqueue_system_audio_samples(
        callback_state,
        started_at_ms,
        samples,
        sample_rate_hz,
        channels,
    )
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::mpsc;
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;

    use super::{
        build_callback_state_for_test, convert_f32_interleaved_to_pcm16_mono,
        format_start_error,
        enqueue_system_audio_samples_for_state_test, invoke_system_audio_callback_for_test,
        MacosSystemAudioDescriptor, MacosSystemCaptureRuntime, StreamPcmConverter,
    };
    use crate::audio::platform::macos::PcmFrameCallback;

    #[test]
    fn convert_f32_interleaved_frame_to_pcm16_mono_downsamples_to_16khz() {
        let descriptor = MacosSystemAudioDescriptor {
            sample_rate_hz: 48_000,
            channels: 2,
        };
        let samples = vec![
            0.25, 0.75, -0.25, 0.25, 0.10, 0.30, 0.20, 0.40, 0.30, 0.50, 0.40, 0.60,
        ];

        let pcm = convert_f32_interleaved_to_pcm16_mono(&descriptor, &samples, 16_000).unwrap();

        assert_eq!(pcm, vec![16_384, 9_830]);
    }

    #[test]
    fn convert_f32_interleaved_resamples_44100hz_to_16khz() {
        let descriptor = MacosSystemAudioDescriptor {
            sample_rate_hz: 44_100,
            channels: 1,
        };
        let samples = vec![0.5; 441];

        let pcm = convert_f32_interleaved_to_pcm16_mono(&descriptor, &samples, 16_000).unwrap();

        assert_eq!(pcm.len(), 160);
        assert_eq!(pcm.first(), Some(&16_384));
        assert_eq!(pcm.last(), Some(&16_384));
    }

    #[test]
    fn stream_resampler_keeps_44100hz_phase_across_512_frame_chunks() {
        let mut converter = StreamPcmConverter::new(16_000);
        let descriptor = MacosSystemAudioDescriptor {
            sample_rate_hz: 44_100,
            channels: 1,
        };
        let samples = vec![0.5; 44_100];
        let mut pcm = Vec::new();

        for chunk in samples.chunks(512) {
            pcm.extend(converter.convert(&descriptor, chunk).unwrap());
        }

        assert_eq!(pcm.len(), 16_000);
        assert!(pcm.iter().all(|sample| *sample == 16_384));
    }

    #[test]
    fn stream_resampler_keeps_48000hz_phase_across_512_frame_chunks() {
        let mut converter = StreamPcmConverter::new(16_000);
        let descriptor = MacosSystemAudioDescriptor {
            sample_rate_hz: 48_000,
            channels: 1,
        };
        let samples = vec![0.5; 48_000];
        let mut pcm = Vec::new();

        for chunk in samples.chunks(512) {
            pcm.extend(converter.convert(&descriptor, chunk).unwrap());
        }

        assert_eq!(pcm.len(), 16_000);
        assert!(pcm.iter().all(|sample| *sample == 16_384));
    }

    #[test]
    fn convert_f32_interleaved_rejects_zero_source_sample_rate() {
        let descriptor = MacosSystemAudioDescriptor {
            sample_rate_hz: 0,
            channels: 2,
        };

        let error =
            convert_f32_interleaved_to_pcm16_mono(&descriptor, &[0.0, 0.0], 16_000).unwrap_err();

        assert_eq!(error, "sample rate must be non-zero");
    }

    #[test]
    fn convert_f32_interleaved_rejects_zero_source_channels() {
        let descriptor = MacosSystemAudioDescriptor {
            sample_rate_hz: 48_000,
            channels: 0,
        };

        let error = convert_f32_interleaved_to_pcm16_mono(&descriptor, &[0.0], 16_000).unwrap_err();

        assert_eq!(error, "source channels must be non-zero");
    }

    #[test]
    fn system_audio_runtime_stop_disarms_callback_before_releasing_state() {
        let (delivered_tx, delivered_rx) = mpsc::channel();
        let callback_count = Arc::new(AtomicUsize::new(0));
        let callback_count_for_sink = callback_count.clone();
        let sink: PcmFrameCallback = Arc::new(move |_, samples| {
            callback_count_for_sink.fetch_add(samples.len(), Ordering::SeqCst);
            delivered_tx.send(()).unwrap();
        });
        let runtime = MacosSystemCaptureRuntime::from_test_sink(sink);

        invoke_system_audio_callback_for_test(&runtime, 1_000, &[0.5; 6], 48_000, 2);
        delivered_rx.recv_timeout(Duration::from_secs(1)).unwrap();
        runtime.stop();
        invoke_system_audio_callback_for_test(&runtime, 1_010, &[0.5; 6], 48_000, 2);

        assert_eq!(callback_count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn system_audio_callback_delivers_on_worker_thread() {
        let callback_thread = thread::current().id();
        let (thread_tx, thread_rx) = mpsc::channel();
        let sink: PcmFrameCallback = Arc::new(move |_, _| {
            thread_tx.send(thread::current().id()).unwrap();
        });
        let runtime = MacosSystemCaptureRuntime::from_test_sink(sink);

        invoke_system_audio_callback_for_test(&runtime, 1_000, &[0.5; 6], 48_000, 2);

        let sink_thread = thread_rx.recv_timeout(Duration::from_secs(1)).unwrap();
        runtime.stop();
        assert_ne!(sink_thread, callback_thread);
    }

    #[test]
    fn system_audio_worker_contains_panicking_sink() {
        let sink: PcmFrameCallback = Arc::new(move |_, _| {
            panic!("system audio sink panic");
        });
        let runtime = MacosSystemCaptureRuntime::from_test_sink(sink);

        invoke_system_audio_callback_for_test(&runtime, 1_000, &[0.5; 6], 48_000, 2);

        runtime.stop();
    }

    #[test]
    fn system_audio_enqueue_drops_when_queue_is_full_without_blocking() {
        let (callback_state, _frame_rx) =
            build_callback_state_for_test(super::SYSTEM_AUDIO_QUEUE_CAPACITY);

        for index in 0..super::SYSTEM_AUDIO_QUEUE_CAPACITY {
            assert!(enqueue_system_audio_samples_for_state_test(
                &callback_state,
                index as u64,
                &[0.5; 6],
                48_000,
                2,
            )
            .is_queued());
        }

        let dropped = enqueue_system_audio_samples_for_state_test(
            &callback_state,
            super::SYSTEM_AUDIO_QUEUE_CAPACITY as u64,
            &[0.5; 6],
            48_000,
            2,
        );

        assert!(dropped.is_dropped());
    }

    #[test]
    fn format_start_error_explains_unsupported_os() {
        let error =
            format_start_error("macOS system audio capture requires ScreenCaptureKit audio capture");

        assert_eq!(
            error,
            "macOS system audio capture requires a supported macOS version with ScreenCaptureKit audio capture."
        );
    }

    #[test]
    fn format_start_error_explains_permission_denied() {
        let error =
            format_start_error("AudioDeviceStart failed with OSStatus -54 ('perm')");

        assert_eq!(
            error,
            "macOS denied system audio capture permission. Grant recording permission and retry. Native error: AudioDeviceStart failed with OSStatus -54 ('perm')"
        );
    }

    #[test]
    fn format_start_error_explains_screen_recording_permission_denied() {
        let error = format_start_error("screen recording permission denied");

        assert_eq!(
            error,
            "macOS denied Screen Recording permission required for system audio capture. Grant Screen Recording permission and retry. Native error: screen recording permission denied"
        );
    }

    #[test]
    fn format_start_error_preserves_native_status_codes() {
        let error = format_start_error(
            "AudioHardwareCreateAggregateDevice failed with OSStatus 1852797029 ('what')",
        );

        assert_eq!(
            error,
            "failed to start macOS system audio capture: AudioHardwareCreateAggregateDevice failed with OSStatus 1852797029 ('what')"
        );
    }
}
