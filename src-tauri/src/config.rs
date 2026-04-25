use std::env;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum MacosSystemAudioMode {
    #[default]
    Disabled,
    MirrorMicrophone,
}

impl MacosSystemAudioMode {
    fn parse_env_value(value: &str) -> Result<Self, String> {
        match value.trim() {
            "" => Ok(Self::Disabled),
            "mirror_microphone" => Ok(Self::MirrorMicrophone),
            other => Err(format!(
                "unsupported MEETING_MACOS_DEV_SYSTEM_AUDIO value: {other}"
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackendRuntimeConfig {
    pub client_id: String,
    pub current_user_id: Option<String>,
    pub current_user_name: Option<String>,
    pub mqtt_broker: Option<String>,
    pub mqtt_username: Option<String>,
    pub mqtt_password: Option<String>,
    pub http_host: String,
    pub http_port: u16,
    pub udp_host: String,
    pub udp_port: u16,
    pub startup_stt_provider: Option<String>,
    pub startup_stt_model: Option<String>,
    pub startup_stt_resource_id: Option<String>,
    pub macos_system_audio_mode: MacosSystemAudioMode,
}

impl Default for BackendRuntimeConfig {
    fn default() -> Self {
        Self {
            client_id: "meeting-desktop".to_string(),
            current_user_id: None,
            current_user_name: None,
            mqtt_broker: None,
            mqtt_username: None,
            mqtt_password: None,
            http_host: "127.0.0.1".to_string(),
            http_port: 8090,
            udp_host: "127.0.0.1".to_string(),
            udp_port: 6000,
            startup_stt_provider: None,
            startup_stt_model: None,
            startup_stt_resource_id: None,
            macos_system_audio_mode: MacosSystemAudioMode::Disabled,
        }
    }
}

impl BackendRuntimeConfig {
    pub fn from_env() -> Result<Self, String> {
        let mut config = Self::default();

        if let Ok(value) = env::var("MEETING_DESKTOP_CLIENT_ID") {
            if !value.trim().is_empty() {
                config.client_id = value;
            }
        }

        if let Ok(value) = env::var("MEETING_USER_ID") {
            if !value.trim().is_empty() {
                config.current_user_id = Some(value);
            }
        }

        if let Ok(value) = env::var("MEETING_USER_NAME") {
            if !value.trim().is_empty() {
                config.current_user_name = Some(value);
            }
        }

        if let Ok(value) = env::var("MEETING_SERVER_MQTT_BROKER") {
            if !value.trim().is_empty() {
                config.mqtt_broker = Some(value);
            }
        }

        if let Ok(value) = env::var("MEETING_SERVER_MQTT_USERNAME") {
            if !value.trim().is_empty() {
                config.mqtt_username = Some(value);
            }
        }

        if let Ok(value) = env::var("MEETING_SERVER_MQTT_PASSWORD") {
            if !value.trim().is_empty() {
                config.mqtt_password = Some(value);
            }
        }

        if let Ok(value) = env::var("MEETING_SERVER_UDP_HOST") {
            if !value.trim().is_empty() {
                config.udp_host = value;
            }
        }

        if let Ok(value) = env::var("MEETING_SERVER_HTTP_HOST") {
            if !value.trim().is_empty() {
                config.http_host = value;
            }
        }

        if let Ok(value) = env::var("MEETING_SERVER_HTTP_PORT") {
            config.http_port = value.parse::<u16>().map_err(|error| error.to_string())?;
        }

        if let Ok(value) = env::var("MEETING_SERVER_UDP_PORT") {
            config.udp_port = value.parse::<u16>().map_err(|error| error.to_string())?;
        }

        if let Ok(value) = env::var("MEETING_STT_PROVIDER") {
            if !value.trim().is_empty() {
                config.startup_stt_provider = Some(value);
            }
        }

        if let Ok(value) = env::var("MEETING_STT_MODEL") {
            if !value.trim().is_empty() {
                config.startup_stt_model = Some(value);
            }
        }

        if let Ok(value) = env::var("MEETING_STT_RESOURCE_ID") {
            if !value.trim().is_empty() {
                config.startup_stt_resource_id = Some(value);
            }
        }

        if let Ok(value) = env::var("MEETING_MACOS_DEV_SYSTEM_AUDIO") {
            config.macos_system_audio_mode = MacosSystemAudioMode::parse_env_value(&value)?;
        }

        Ok(config)
    }

    pub fn udp_target_addr(&self) -> String {
        format!("{}:{}", self.udp_host, self.udp_port)
    }

    pub fn admin_api_base_url(&self) -> String {
        format!("http://{}:{}", self.http_host, self.http_port)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Mutex, OnceLock};

    use super::{BackendRuntimeConfig, MacosSystemAudioMode};

    static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    #[test]
    fn default_runtime_config_targets_local_backend() {
        let config = BackendRuntimeConfig::default();

        assert_eq!(config.client_id, "meeting-desktop");
        assert_eq!(config.mqtt_broker, None);
        assert_eq!(config.http_host, "127.0.0.1");
        assert_eq!(config.http_port, 8090);
        assert_eq!(config.udp_host, "127.0.0.1");
        assert_eq!(config.udp_port, 6000);
        assert_eq!(config.startup_stt_provider, None);
        assert_eq!(config.current_user_id, None);
        assert_eq!(config.current_user_name, None);
        assert_eq!(
            config.macos_system_audio_mode,
            MacosSystemAudioMode::Disabled
        );
    }

    #[test]
    fn runtime_config_reads_env_overrides() {
        let _guard = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();

        std::env::set_var("MEETING_DESKTOP_CLIENT_ID", "desktop-dev");
        std::env::set_var("MEETING_SERVER_MQTT_BROKER", "tcp://127.0.0.1:1883");
        std::env::set_var("MEETING_SERVER_MQTT_USERNAME", "meeting-user");
        std::env::set_var("MEETING_SERVER_MQTT_PASSWORD", "meeting-pass");
        std::env::set_var("MEETING_SERVER_HTTP_HOST", "192.168.1.10");
        std::env::set_var("MEETING_SERVER_HTTP_PORT", "8091");
        std::env::set_var("MEETING_SERVER_UDP_HOST", "192.168.1.5");
        std::env::set_var("MEETING_SERVER_UDP_PORT", "7002");
        std::env::set_var("MEETING_STT_PROVIDER", "volcengine_streaming");
        std::env::set_var("MEETING_STT_MODEL", "bigmodel");
        std::env::set_var("MEETING_STT_RESOURCE_ID", "volc.seedasr.sauc.duration");
        std::env::set_var("MEETING_USER_ID", "user-1");
        std::env::set_var("MEETING_USER_NAME", "张三");
        std::env::set_var("MEETING_MACOS_DEV_SYSTEM_AUDIO", "mirror_microphone");

        let config = BackendRuntimeConfig::from_env().unwrap();

        assert_eq!(config.client_id, "desktop-dev");
        assert_eq!(config.mqtt_broker.as_deref(), Some("tcp://127.0.0.1:1883"));
        assert_eq!(config.mqtt_username.as_deref(), Some("meeting-user"));
        assert_eq!(config.mqtt_password.as_deref(), Some("meeting-pass"));
        assert_eq!(config.http_host, "192.168.1.10");
        assert_eq!(config.http_port, 8091);
        assert_eq!(config.udp_host, "192.168.1.5");
        assert_eq!(config.udp_port, 7002);
        assert_eq!(
            config.startup_stt_provider.as_deref(),
            Some("volcengine_streaming")
        );
        assert_eq!(config.startup_stt_model.as_deref(), Some("bigmodel"));
        assert_eq!(
            config.startup_stt_resource_id.as_deref(),
            Some("volc.seedasr.sauc.duration")
        );
        assert_eq!(config.current_user_id.as_deref(), Some("user-1"));
        assert_eq!(config.current_user_name.as_deref(), Some("张三"));
        assert_eq!(
            config.macos_system_audio_mode,
            MacosSystemAudioMode::MirrorMicrophone
        );

        std::env::remove_var("MEETING_DESKTOP_CLIENT_ID");
        std::env::remove_var("MEETING_SERVER_MQTT_BROKER");
        std::env::remove_var("MEETING_SERVER_MQTT_USERNAME");
        std::env::remove_var("MEETING_SERVER_MQTT_PASSWORD");
        std::env::remove_var("MEETING_SERVER_HTTP_HOST");
        std::env::remove_var("MEETING_SERVER_HTTP_PORT");
        std::env::remove_var("MEETING_SERVER_UDP_HOST");
        std::env::remove_var("MEETING_SERVER_UDP_PORT");
        std::env::remove_var("MEETING_STT_PROVIDER");
        std::env::remove_var("MEETING_STT_MODEL");
        std::env::remove_var("MEETING_STT_RESOURCE_ID");
        std::env::remove_var("MEETING_USER_ID");
        std::env::remove_var("MEETING_USER_NAME");
        std::env::remove_var("MEETING_MACOS_DEV_SYSTEM_AUDIO");
    }

    #[test]
    fn runtime_config_formats_udp_target_address() {
        let config = BackendRuntimeConfig {
            client_id: "desktop-dev".to_string(),
            current_user_id: Some("user-2".to_string()),
            current_user_name: Some("李四".to_string()),
            mqtt_broker: Some("tcp://127.0.0.1:1883".to_string()),
            mqtt_username: None,
            mqtt_password: None,
            http_host: "127.0.0.1".to_string(),
            http_port: 8090,
            udp_host: "10.0.0.8".to_string(),
            udp_port: 7008,
            startup_stt_provider: None,
            startup_stt_model: None,
            startup_stt_resource_id: None,
            macos_system_audio_mode: MacosSystemAudioMode::Disabled,
        };

        assert_eq!(config.udp_target_addr(), "10.0.0.8:7008");
    }
}
