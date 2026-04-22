use serde::Deserialize;

use crate::events::bus::EventBus;
use crate::events::types::{RuntimeEvent, SummaryDeltaPayload, TranscriptDeltaPayload};
use crate::protocol::topics::{
    action_items_topic, control_reply_topic, events_topic, stt_topic, summary_topic,
};
use crate::transport::control_transport::ControlTransport;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MqttControlConfig {
    pub client_id: String,
    pub session_id: String,
}

#[derive(Debug, Clone, Default)]
pub struct MqttControlTransport {
    connected: bool,
}

impl MqttControlTransport {
    pub fn new() -> Self {
        Self { connected: false }
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
        let _ = self.connected;
        Ok(())
    }

    fn disconnect(&self) -> Result<(), String> {
        Ok(())
    }

    fn open_session(&self) -> Result<(), String> {
        Ok(())
    }

    fn close_session(&self) -> Result<(), String> {
        Ok(())
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
}
