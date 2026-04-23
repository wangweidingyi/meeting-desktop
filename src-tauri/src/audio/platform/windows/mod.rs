use std::fmt;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::thread::JoinHandle;

pub mod device_enumerator;
pub mod format;
pub mod loopback_capture;
pub mod mic_capture;
pub mod runtime_sink;
mod worker;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioDeviceDescriptor {
    pub id: String,
    pub name: String,
    pub is_default: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureStreamDescriptor {
    pub device: AudioDeviceDescriptor,
    pub sample_rate_hz: u32,
    pub channels: u16,
}

pub type PcmFrameCallback = Arc<dyn Fn(u64, Vec<i16>) + Send + Sync + 'static>;

pub struct WindowsCaptureHandle {
    descriptor: CaptureStreamDescriptor,
    sink: PcmFrameCallback,
    active: Arc<AtomicBool>,
    worker: Mutex<Option<JoinHandle<()>>>,
}

impl WindowsCaptureHandle {
    pub fn new(descriptor: CaptureStreamDescriptor, sink: PcmFrameCallback) -> Self {
        Self {
            descriptor,
            sink,
            active: Arc::new(AtomicBool::new(true)),
            worker: Mutex::new(None),
        }
    }

    pub fn with_worker(
        descriptor: CaptureStreamDescriptor,
        sink: PcmFrameCallback,
        active: Arc<AtomicBool>,
        worker: JoinHandle<()>,
    ) -> Self {
        Self {
            descriptor,
            sink,
            active,
            worker: Mutex::new(Some(worker)),
        }
    }

    pub fn descriptor(&self) -> &CaptureStreamDescriptor {
        &self.descriptor
    }

    pub fn sink(&self) -> PcmFrameCallback {
        self.sink.clone()
    }

    pub fn is_active(&self) -> bool {
        self.active.load(Ordering::SeqCst)
    }

    pub fn stop(&self) {
        self.active.store(false, Ordering::SeqCst);
        if let Ok(mut worker) = self.worker.lock() {
            if let Some(worker) = worker.take() {
                let _ = worker.join();
            }
        }
    }

    pub fn active_flag(&self) -> Arc<AtomicBool> {
        self.active.clone()
    }
}

impl fmt::Debug for WindowsCaptureHandle {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("WindowsCaptureHandle")
            .field("descriptor", &self.descriptor)
            .field("active", &self.is_active())
            .finish_non_exhaustive()
    }
}

#[derive(Debug)]
pub struct WindowsCaptureRuntime {
    microphone: WindowsCaptureHandle,
    loopback: WindowsCaptureHandle,
}

impl WindowsCaptureRuntime {
    pub fn new(microphone: WindowsCaptureHandle, loopback: WindowsCaptureHandle) -> Self {
        Self {
            microphone,
            loopback,
        }
    }

    pub fn microphone(&self) -> &WindowsCaptureHandle {
        &self.microphone
    }

    pub fn loopback(&self) -> &WindowsCaptureHandle {
        &self.loopback
    }

    pub fn stop(&self) {
        self.microphone.stop();
        self.loopback.stop();
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::AtomicUsize;
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;

    use super::{
        AudioDeviceDescriptor, CaptureStreamDescriptor, PcmFrameCallback, WindowsCaptureHandle,
        WindowsCaptureRuntime,
    };

    fn noop_sink() -> PcmFrameCallback {
        Arc::new(|_, _| {})
    }

    fn descriptor(label: &str) -> CaptureStreamDescriptor {
        CaptureStreamDescriptor {
            device: AudioDeviceDescriptor {
                id: format!("{label}-id"),
                name: label.to_string(),
                is_default: true,
            },
            sample_rate_hz: 16_000,
            channels: 1,
        }
    }

    #[test]
    fn capture_handle_starts_active_and_can_be_stopped() {
        let handle = WindowsCaptureHandle::new(descriptor("mic"), noop_sink());

        assert!(handle.is_active());

        handle.stop();

        assert!(!handle.is_active());
    }

    #[test]
    fn capture_runtime_stop_stops_both_handles() {
        let runtime = WindowsCaptureRuntime::new(
            WindowsCaptureHandle::new(descriptor("mic"), noop_sink()),
            WindowsCaptureHandle::new(descriptor("loopback"), noop_sink()),
        );

        runtime.stop();

        assert!(!runtime.microphone().is_active());
        assert!(!runtime.loopback().is_active());
    }

    #[test]
    fn capture_handle_stop_waits_for_worker_thread_exit() {
        let stopped = Arc::new(AtomicUsize::new(0));
        let handle = {
            let stopped = stopped.clone();
            let descriptor = descriptor("mic");
            let sink = noop_sink();
            let active = Arc::new(std::sync::atomic::AtomicBool::new(true));
            let worker_active = active.clone();
            let worker = thread::spawn(move || {
                while worker_active.load(std::sync::atomic::Ordering::SeqCst) {
                    thread::sleep(Duration::from_millis(5));
                }
                stopped.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            });

            WindowsCaptureHandle::with_worker(descriptor, sink, active, worker)
        };

        handle.stop();

        assert_eq!(stopped.load(std::sync::atomic::Ordering::SeqCst), 1);
        assert!(!handle.is_active());
    }
}
