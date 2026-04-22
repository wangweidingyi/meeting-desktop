use std::sync::Mutex;

use serde::Deserialize;

use crate::events::bus::EventBus;
use crate::events::types::{RuntimeEvent, SummaryDeltaPayload, TranscriptDeltaPayload};
use crate::protocol::messages::{
    AudioFormat, FeatureFlags, MessageEnvelope, MessageType, SessionHelloPayload,
    TransportSelection,
};
use crate::protocol::topics::{
    action_items_topic, control_reply_topic, events_topic, stt_topic, summary_topic,
};
use crate::transport::control_transport::ControlTransport;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MqttControlConfig {
    pub client_id: String,
    pub session_id: String,
}

#[derive(Debug)]
pub struct MqttControlTransport {
    config: MqttControlConfig,
    state: Mutex<TransportState>,
}

#[derive(Debug, Default)]
struct TransportState {
    connected: bool,
    outbound_messages: Vec<String>,
    last_error: Option<String>,
}

impl MqttControlTransport {
    pub fn new(config: MqttControlConfig) -> Self {
        Self {
            config,
            state: Mutex::new(TransportState::default()),
        }
    }

    pub fn subscription_topics(config: &MqttControlConfig) -> Vec<String> {
        vec![
            control_reply_topic(&config.client_id, &config.session_id),
            events_topic(&config.client_id, &config.session_id),
            stt_topic(&config.client_id, &config.session_id),
            summary_topic(&config.client_id, &config.session_id),
            action_items_topic(&config.client_id, &config.session_id),
        ]
    }

    pub fn queued_messages(&self) -> Result<Vec<String>, String> {
        let state = self.state.lock().map_err(|error| error.to_string())?;
        Ok(state.outbound_messages.clone())
    }

    fn is_connected(&self) -> Result<bool, String> {
        let state = self.state.lock().map_err(|error| error.to_string())?;
        Ok(state.connected)
    }

    pub fn dispatch_message(event_bus: &EventBus, raw_payload: &str) -> Result<(), String> {
        let envelope: IncomingEnvelope =
            serde_json::from_str(raw_payload).map_err(|error| error.to_string())?;

        match envelope.message_type.as_str() {
            "stt_delta" => {
                let payload = deserialize_payload::<IncomingTranscriptPayload>(envelope.payload)?;
                event_bus.publish(RuntimeEvent::TranscriptDelta(TranscriptDeltaPayload {
                    text: payload.text,
                    is_final: payload.is_final,
                }))
            }
            "summary_delta" => {
                let payload = deserialize_payload::<IncomingSummaryPayload>(envelope.payload)?;
                event_bus.publish(RuntimeEvent::SummaryDelta(SummaryDeltaPayload {
                    text: payload.text,
                }))
            }
            "heartbeat" => event_bus.publish(RuntimeEvent::Heartbeat {
                session_id: envelope.session_id,
            }),
            "error" => {
                let payload = deserialize_payload::<IncomingErrorPayload>(envelope.payload)?;
                event_bus.publish(RuntimeEvent::TransportError {
                    message: payload.message,
                })
            }
            _ => Ok(()),
        }
    }
}

impl ControlTransport for MqttControlTransport {
    fn connect(&self) -> Result<(), String> {
        let mut state = self.state.lock().map_err(|error| error.to_string())?;
        state.connected = true;
        Ok(())
    }

    fn disconnect(&self) -> Result<(), String> {
        let mut state = self.state.lock().map_err(|error| error.to_string())?;
        state.connected = false;
        Ok(())
    }

    fn open_session(&self, title: &str) -> Result<String, String> {
        if !self.is_connected()? {
            return Err("transport is not connected".to_string());
        }

        let payload = SessionHelloPayload {
            audio: AudioFormat {
                encoding: "pcm_s16le".to_string(),
                sample_rate: 16_000,
                channels: 1,
            },
            transport: TransportSelection {
                control: "mqtt".to_string(),
                audio: "udp".to_string(),
            },
            features: FeatureFlags {
                realtime_transcript: true,
                realtime_summary: true,
                action_items: true,
            },
            title: title.to_string(),
        };

        let message = MessageEnvelope {
            version: "v1".to_string(),
            message_id: format!("{}-hello-{}", self.config.session_id, self.config.client_id),
            correlation_id: None,
            client_id: self.config.client_id.clone(),
            session_id: self.config.session_id.clone(),
            seq: 1,
            sent_at: "1970-01-01T00:00:00Z".to_string(),
            message_type: MessageType::SessionHello,
            payload,
        };

        let serialized = serde_json::to_string(&message).map_err(|error| error.to_string())?;
        self.send_control_message(&serialized)?;
        Ok(serialized)
    }

    fn close_session(&self) -> Result<String, String> {
        if !self.is_connected()? {
            return Err("transport is not connected".to_string());
        }

        let message = serde_json::json!({
            "version": "v1",
            "messageId": format!("{}-close", self.config.session_id),
            "clientId": self.config.client_id.clone(),
            "sessionId": self.config.session_id.clone(),
            "seq": 999_u64,
            "sentAt": "1970-01-01T00:00:00Z",
            "type": "session/close",
            "payload": {}
        })
        .to_string();

        self.send_control_message(&message)?;
        Ok(message)
    }

    fn send_control_message(&self, payload: &str) -> Result<(), String> {
        let mut state = self.state.lock().map_err(|error| error.to_string())?;
        if !state.connected {
            return Err("transport is not connected".to_string());
        }
        state.outbound_messages.push(payload.to_string());
        Ok(())
    }

    fn on_message(&self, event_bus: &EventBus, payload: &str) -> Result<(), String> {
        Self::dispatch_message(event_bus, payload)
    }

    fn on_error(&self, event_bus: &EventBus, message: &str) -> Result<(), String> {
        {
            let mut state = self.state.lock().map_err(|error| error.to_string())?;
            state.last_error = Some(message.to_string());
        }

        event_bus.publish(RuntimeEvent::TransportError {
            message: message.to_string(),
        })
    }
}

#[derive(Debug, Deserialize)]
struct IncomingEnvelope {
    #[serde(rename = "type")]
    message_type: String,
    #[serde(rename = "sessionId")]
    session_id: String,
    payload: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct IncomingTranscriptPayload {
    text: String,
    #[serde(rename = "isFinal", default)]
    is_final: bool,
}

#[derive(Debug, Deserialize)]
struct IncomingSummaryPayload {
    text: String,
}

#[derive(Debug, Deserialize)]
struct IncomingErrorPayload {
    message: String,
}

fn deserialize_payload<T>(payload: serde_json::Value) -> Result<T, String>
where
    T: for<'de> Deserialize<'de>,
{
    serde_json::from_value(payload).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use crate::events::bus::EventBus;
    use crate::events::types::RuntimeEvent;
    use crate::transport::control_transport::ControlTransport;

    use super::{MqttControlConfig, MqttControlTransport};

    #[test]
    fn subscription_topics_cover_control_and_streaming_channels() {
        let topics = MqttControlTransport::subscription_topics(&MqttControlConfig {
            client_id: "client-a".to_string(),
            session_id: "session-1".to_string(),
        });

        assert_eq!(
            topics,
            vec![
                "meetings/client-a/session/session-1/control/reply",
                "meetings/client-a/session/session-1/events",
                "meetings/client-a/session/session-1/stt",
                "meetings/client-a/session/session-1/summary",
                "meetings/client-a/session/session-1/action-items",
            ]
        );
    }

    #[test]
    fn dispatch_message_emits_transcript_delta_event() {
        let event_bus = EventBus::default();

        MqttControlTransport::dispatch_message(
            &event_bus,
            r#"{
                "type": "stt_delta",
                "sessionId": "session-1",
                "payload": {
                    "text": "这是新的转写片段",
                    "isFinal": false
                }
            }"#,
        )
        .unwrap();

        let events = event_bus.snapshot().unwrap();
        assert_eq!(
            events,
            vec![RuntimeEvent::TranscriptDelta(
                crate::events::types::TranscriptDeltaPayload {
                    text: "这是新的转写片段".to_string(),
                    is_final: false,
                }
            )]
        );
    }

    #[test]
    fn open_session_serializes_hello_envelope_and_queues_it() {
        let transport = MqttControlTransport::new(MqttControlConfig {
            client_id: "client-a".to_string(),
            session_id: "session-1".to_string(),
        });

        transport.connect().unwrap();
        let hello = transport.open_session("架构评审会").unwrap();

        assert!(hello.contains("\"type\":\"session/hello\""));
        assert!(hello.contains("\"title\":\"架构评审会\""));
        assert_eq!(transport.queued_messages().unwrap(), vec![hello]);
    }

    #[test]
    fn on_error_emits_transport_error_event() {
        let transport = MqttControlTransport::new(MqttControlConfig {
            client_id: "client-a".to_string(),
            session_id: "session-1".to_string(),
        });
        let event_bus = EventBus::default();

        transport
            .on_error(&event_bus, "broker connection lost")
            .unwrap();

        assert_eq!(
            event_bus.snapshot().unwrap(),
            vec![RuntimeEvent::TransportError {
                message: "broker connection lost".to_string(),
            }]
        );
    }
}
