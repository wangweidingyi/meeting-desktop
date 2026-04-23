pub mod windows;

#[cfg(target_os = "macos")]
pub mod macos;

pub enum PlatformCaptureRuntime {
    #[cfg(target_os = "windows")]
    Windows(windows::WindowsCaptureRuntime),
    #[cfg(target_os = "macos")]
    Macos(macos::MacosCaptureRuntime),
}

impl PlatformCaptureRuntime {
    pub fn stop(&self) {
        match self {
            #[cfg(target_os = "windows")]
            Self::Windows(runtime) => runtime.stop(),
            #[cfg(target_os = "macos")]
            Self::Macos(runtime) => runtime.stop(),
        }
    }
}
