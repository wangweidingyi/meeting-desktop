use super::{
    format::CapturedSampleFormat,
    worker::{start_capture_worker, DeviceRole, WasapiCaptureConfig},
    AudioDeviceDescriptor, CaptureStreamDescriptor, PcmFrameCallback, WindowsCaptureHandle,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowsLoopbackCapture {
    descriptor: CaptureStreamDescriptor,
}

impl WindowsLoopbackCapture {
    pub fn new(device: AudioDeviceDescriptor) -> Self {
        Self {
            descriptor: CaptureStreamDescriptor {
                device,
                sample_rate_hz: 48_000,
                channels: 2,
            },
        }
    }

    pub fn descriptor(&self) -> &CaptureStreamDescriptor {
        &self.descriptor
    }

    pub fn start(&self) -> Result<(), String> {
        Ok(())
    }

    pub fn start_with_sink(&self, sink: PcmFrameCallback) -> Result<WindowsCaptureHandle, String> {
        start_capture_worker(
            self.descriptor.clone(),
            sink,
            WasapiCaptureConfig {
                role: DeviceRole::Loopback,
                sample_format: CapturedSampleFormat::F32,
                chunk_frames: 4_800,
            },
        )
    }
}
