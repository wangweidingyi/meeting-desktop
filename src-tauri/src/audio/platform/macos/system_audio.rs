use std::ffi::{c_char, c_void, CStr};
use std::ptr;
use std::slice;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use super::PcmFrameCallback;

const TARGET_SAMPLE_RATE_HZ: u32 = 16_000;
const ERROR_BUFFER_LEN: usize = 1024;

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
}

struct BridgeCallbackState {
    active: AtomicBool,
    sink: PcmFrameCallback,
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
        start_core_audio_system_tap(sink)
    }
}

impl MacosSystemCaptureRuntime {
    pub fn stop(&self) {
        self.inner.stop();
    }

    #[cfg(test)]
    fn from_test_sink(sink: PcmFrameCallback) -> Self {
        Self {
            inner: Arc::new(MacosSystemCaptureHandle {
                callback_state: Arc::new(BridgeCallbackState {
                    active: AtomicBool::new(true),
                    sink,
                }),
                native_handle: Mutex::new(None),
                native_callback_state: Mutex::new(None),
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
    if descriptor.sample_rate_hz % target_sample_rate_hz != 0 {
        return Err("source sample rate must be an integer multiple of target rate".to_string());
    }

    let mono = downmix_f32_interleaved_to_mono(descriptor, samples);
    let ratio = descriptor.sample_rate_hz / target_sample_rate_hz;
    let downsampled = if ratio <= 1 {
        mono
    } else {
        mono.into_iter().step_by(ratio as usize).collect()
    };

    Ok(downsampled
        .into_iter()
        .map(|sample| {
            let clamped = sample.clamp(-1.0, 1.0);
            (clamped * f32::from(i16::MAX)).round() as i16
        })
        .collect())
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

fn start_core_audio_system_tap(
    sink: PcmFrameCallback,
) -> Result<MacosSystemCaptureRuntime, String> {
    let callback_state = Arc::new(BridgeCallbackState {
        active: AtomicBool::new(true),
        sink,
    });
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
        return Err(read_error_buffer(&error_buffer));
    }

    Ok(MacosSystemCaptureRuntime {
        inner: Arc::new(MacosSystemCaptureHandle {
            callback_state,
            native_handle: Mutex::new(Some(native_handle)),
            native_callback_state: Mutex::new(Some(native_callback_state)),
        }),
    })
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
    forward_system_audio_samples(
        callback_state,
        started_at_ms,
        slice::from_raw_parts(samples, sample_count),
        sample_rate_hz,
        channels,
    );
}

fn forward_system_audio_samples(
    callback_state: &BridgeCallbackState,
    started_at_ms: u64,
    samples: &[f32],
    sample_rate_hz: u32,
    channels: u16,
) {
    if !callback_state.active.load(Ordering::SeqCst) {
        return;
    }

    let descriptor = MacosSystemAudioDescriptor {
        sample_rate_hz,
        channels,
    };
    let pcm =
        match convert_f32_interleaved_to_pcm16_mono(&descriptor, samples, TARGET_SAMPLE_RATE_HZ) {
            Ok(pcm) if !pcm.is_empty() => pcm,
            _ => return,
        };

    if callback_state.active.load(Ordering::SeqCst) {
        (callback_state.sink)(started_at_ms, pcm);
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

#[cfg(test)]
fn invoke_system_audio_callback_for_test(
    runtime: &MacosSystemCaptureRuntime,
    started_at_ms: u64,
    samples: &[f32],
    sample_rate_hz: u32,
    channels: u16,
) {
    forward_system_audio_samples(
        &runtime.inner.callback_state,
        started_at_ms,
        samples,
        sample_rate_hz,
        channels,
    );
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    use super::{
        convert_f32_interleaved_to_pcm16_mono, invoke_system_audio_callback_for_test,
        MacosSystemAudioDescriptor, MacosSystemCaptureRuntime,
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
    fn convert_f32_interleaved_rejects_non_integer_downsample_ratio() {
        let descriptor = MacosSystemAudioDescriptor {
            sample_rate_hz: 44_100,
            channels: 2,
        };

        let error =
            convert_f32_interleaved_to_pcm16_mono(&descriptor, &[0.0, 0.0], 16_000).unwrap_err();

        assert_eq!(
            error,
            "source sample rate must be an integer multiple of target rate"
        );
    }

    #[test]
    fn system_audio_runtime_stop_disarms_callback_before_releasing_state() {
        let callback_count = Arc::new(AtomicUsize::new(0));
        let callback_count_for_sink = callback_count.clone();
        let sink: PcmFrameCallback = Arc::new(move |_, samples| {
            callback_count_for_sink.fetch_add(samples.len(), Ordering::SeqCst);
        });
        let runtime = MacosSystemCaptureRuntime::from_test_sink(sink);

        invoke_system_audio_callback_for_test(&runtime, 1_000, &[0.5, 0.5], 48_000, 2);
        runtime.stop();
        invoke_system_audio_callback_for_test(&runtime, 1_010, &[0.5, 0.5], 48_000, 2);

        assert_eq!(callback_count.load(Ordering::SeqCst), 1);
    }
}
