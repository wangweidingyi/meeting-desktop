use std::net::SocketAddr;
use std::sync::Arc;

use crate::config::BackendRuntimeConfig;
use crate::transport::audio_transport::{AudioTransport, AudioUploadProgress};
use crate::transport::control_transport::ControlTransport;
use crate::transport::mqtt_control::{
    MqttControlConfig, MqttControlTransport, RumqttcBrokerClient, RumqttcBrokerClientConfig,
};
use crate::transport::udp_audio::{NetworkUdpSocket, UdpAudioTransport};
use crate::{audio::AudioChunk, events::bus::EventBus};

#[derive(Debug)]
pub enum ControlTransportRuntime {
    Mqtt(MqttControlTransport),
}

impl ControlTransport for ControlTransportRuntime {
    fn connect(&self) -> Result<(), String> {
        match self {
            Self::Mqtt(transport) => transport.connect(),
        }
    }

    fn disconnect(&self) -> Result<(), String> {
        match self {
            Self::Mqtt(transport) => transport.disconnect(),
        }
    }

    fn open_session(&self, title: &str) -> Result<String, String> {
        match self {
            Self::Mqtt(transport) => transport.open_session(title),
        }
    }

    fn close_session(&self) -> Result<String, String> {
        match self {
            Self::Mqtt(transport) => transport.close_session(),
        }
    }

    fn send_control_message(&self, payload: &str) -> Result<(), String> {
        match self {
            Self::Mqtt(transport) => transport.send_control_message(payload),
        }
    }

    fn on_message(&self, event_bus: &EventBus, payload: &str) -> Result<(), String> {
        match self {
            Self::Mqtt(transport) => transport.on_message(event_bus, payload),
        }
    }

    fn on_error(&self, event_bus: &EventBus, message: &str) -> Result<(), String> {
        match self {
            Self::Mqtt(transport) => transport.on_error(event_bus, message),
        }
    }
}

impl ControlTransportRuntime {
    pub fn start_recording(&self) -> Result<String, String> {
        match self {
            Self::Mqtt(transport) => transport.start_recording(),
        }
    }

    pub fn pause_recording(&self) -> Result<String, String> {
        match self {
            Self::Mqtt(transport) => transport.pause_recording(),
        }
    }

    pub fn resume_recording(&self) -> Result<String, String> {
        match self {
            Self::Mqtt(transport) => transport.resume_recording(),
        }
    }

    pub fn stop_recording(&self) -> Result<String, String> {
        match self {
            Self::Mqtt(transport) => transport.stop_recording(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum AudioTransportRuntime {
    Udp(UdpAudioTransport<NetworkUdpSocket>),
}

impl AudioTransport for AudioTransportRuntime {
    fn send_audio_chunk(&self, chunk: &AudioChunk) -> Result<AudioUploadProgress, String> {
        match self {
            Self::Udp(transport) => transport.send_audio_chunk(chunk),
        }
    }
}

#[derive(Debug)]
pub struct SessionTransportRuntime {
    control: ControlTransportRuntime,
    audio: AudioTransportRuntime,
    audio_target_addr: String,
}

impl SessionTransportRuntime {
    pub fn control_transport(&self) -> &ControlTransportRuntime {
        &self.control
    }

    pub fn audio_transport(&self) -> &AudioTransportRuntime {
        &self.audio
    }

    pub fn control_config(&self) -> &MqttControlConfig {
        match &self.control {
            ControlTransportRuntime::Mqtt(transport) => transport.config(),
        }
    }

    pub fn audio_target_addr(&self) -> &str {
        &self.audio_target_addr
    }

    pub fn audio_socket_peer_addr(&self) -> Result<SocketAddr, String> {
        match &self.audio {
            AudioTransportRuntime::Udp(transport) => transport.sink().peer_addr(),
        }
    }
}

#[derive(Debug, Default)]
pub struct SessionTransportFactory;

impl SessionTransportFactory {
    pub fn prepare(
        config: &BackendRuntimeConfig,
        session_id: &str,
        event_bus: EventBus,
    ) -> Result<SessionTransportRuntime, String> {
        let control_config = MqttControlConfig {
            client_id: config.client_id.clone(),
            session_id: session_id.to_string(),
        };
        let control = if let Some(broker_url) = &config.mqtt_broker {
            ControlTransportRuntime::Mqtt(MqttControlTransport::with_broker_client(
                control_config,
                event_bus,
                Arc::new(RumqttcBrokerClient::new(RumqttcBrokerClientConfig {
                    broker_url: broker_url.clone(),
                    client_id: format!("{}-{}", config.client_id, session_id),
                    username: config.mqtt_username.clone(),
                    password: config.mqtt_password.clone(),
                    keep_alive_secs: 30,
                })),
            ))
        } else {
            ControlTransportRuntime::Mqtt(MqttControlTransport::new(control_config, event_bus))
        };
        let audio_target_addr = config.udp_target_addr();
        let audio = AudioTransportRuntime::Udp(UdpAudioTransport::new(
            session_id,
            NetworkUdpSocket::connect(&audio_target_addr)?,
        ));

        Ok(SessionTransportRuntime {
            control,
            audio,
            audio_target_addr,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::net::{SocketAddr, UdpSocket};

    use crate::config::{BackendRuntimeConfig, MacosSystemAudioMode};
    use crate::transport::test_support::lock_network_test;

    use super::SessionTransportFactory;

    #[test]
    #[ignore = "requires local UDP socket permissions"]
    fn session_transport_factory_prepares_control_and_audio_runtime() {
        let _network_guard = lock_network_test();
        let listener = UdpSocket::bind("127.0.0.1:0").unwrap();
        let target = listener.local_addr().unwrap();
        let config = BackendRuntimeConfig {
            client_id: "desktop-dev".to_string(),
            mqtt_broker: Some("tcp://127.0.0.1:1883".to_string()),
            mqtt_username: Some("meeting-user".to_string()),
            mqtt_password: Some("meeting-pass".to_string()),
            udp_host: "127.0.0.1".to_string(),
            udp_port: target.port(),
            macos_system_audio_mode: MacosSystemAudioMode::Disabled,
        };

        let runtime = SessionTransportFactory::prepare(
            &config,
            "session-123",
            crate::events::bus::EventBus::default(),
        )
        .unwrap();

        assert_eq!(runtime.control_config().client_id, "desktop-dev");
        assert_eq!(runtime.control_config().session_id, "session-123");
        assert_eq!(
            runtime.audio_target_addr(),
            format!("127.0.0.1:{}", target.port())
        );
        assert_eq!(
            runtime.audio_socket_peer_addr().unwrap(),
            SocketAddr::from(([127, 0, 0, 1], target.port()))
        );
    }
}
