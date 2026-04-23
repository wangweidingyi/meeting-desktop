use super::AudioDeviceDescriptor;

#[derive(Debug, Default)]
pub struct WindowsAudioDeviceEnumerator;

impl WindowsAudioDeviceEnumerator {
    pub fn list_microphones(&self) -> Vec<AudioDeviceDescriptor> {
        vec![AudioDeviceDescriptor {
            id: "windows-default-mic".to_string(),
            name: "Default Microphone".to_string(),
            is_default: true,
        }]
    }

    pub fn default_loopback_device(&self) -> Option<AudioDeviceDescriptor> {
        Some(AudioDeviceDescriptor {
            id: "windows-default-loopback".to_string(),
            name: "Default Speaker Loopback".to_string(),
            is_default: true,
        })
    }
}
