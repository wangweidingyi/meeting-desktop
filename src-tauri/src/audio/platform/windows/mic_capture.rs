use super::{
    format::CapturedSampleFormat,
    worker::{start_capture_worker, DeviceRole, WasapiCaptureConfig},
    AudioDeviceDescriptor, CaptureStreamDescriptor, PcmFrameCallback, WindowsCaptureHandle,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowsMicrophoneCapture {
    descriptor: CaptureStreamDescriptor,
}

impl WindowsMicrophoneCapture {
    pub fn new(device: AudioDeviceDescriptor) -> Self {
        Self {
            descriptor: CaptureStreamDescriptor {
                device,
                sample_rate_hz: 16_000,
                channels: 1,
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
                role: DeviceRole::Microphone,
                sample_format: CapturedSampleFormat::I16,
                chunk_frames: 1_600,
            },
        )
    }
}
