use std::fmt;
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use rumqttc::{Client, Event, Incoming, MqttOptions, Outgoing, QoS};
use serde::Deserialize;

use crate::events::bus::EventBus;
use crate::events::types::{
    ActionItemsDeltaPayload, RuntimeEvent, SummaryDeltaPayload, TranscriptDeltaPayload,
    TransportConnectionState, TransportStatePayload,
};
use crate::protocol::messages::{
    AudioFormat, FeatureFlags, MessageEnvelope, MessageType, SessionHelloPayload,
    TransportSelection,
};
use crate::protocol::topics::{
    action_items_topic, control_reply_topic, control_topic, events_topic, stt_topic, summary_topic,
};
use crate::transport::control_transport::ControlTransport;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MqttControlConfig {
    pub client_id: String,
    pub session_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrokerIncomingMessage {
    pub topic: String,
    pub payload: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrokerQos {
    AtMostOnce,
    AtLeastOnce,
}

impl BrokerQos {
    fn to_rumqttc(self) -> QoS {
        match self {
            Self::AtMostOnce => QoS::AtMostOnce,
            Self::AtLeastOnce => QoS::AtLeastOnce,
        }
    }
}

pub type MessageHandler = Arc<dyn Fn(BrokerIncomingMessage) + Send + Sync + 'static>;
pub type LifecycleHandler = Arc<dyn Fn(BrokerLifecycleEvent) + Send + Sync + 'static>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BrokerLifecycleEvent {
    Connecting,
    Connected,
    Reconnecting { message: String },
    Disconnected { message: Option<String> },
}

pub trait MqttBrokerClient: Send + Sync {
    fn connect(
        &self,
        handler: MessageHandler,
        lifecycle_handler: LifecycleHandler,
    ) -> Result<(), String>;
    fn disconnect(&self) -> Result<(), String>;
    fn subscribe(&self, topic: &str, qos: BrokerQos) -> Result<(), String>;
    fn publish(
        &self,
        topic: &str,
        qos: BrokerQos,
        retained: bool,
        payload: Vec<u8>,
    ) -> Result<(), String>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RumqttcBrokerClientConfig {
    pub broker_url: String,
    pub client_id: String,
    pub username: Option<String>,
    pub password: Option<String>,
    pub keep_alive_secs: u64,
}

pub struct RumqttcBrokerClient {
    config: RumqttcBrokerClientConfig,
    runtime: Mutex<Option<RumqttcRuntime>>,
}

struct RumqttcRuntime {
    client: Client,
    listener: JoinHandle<()>,
}

impl RumqttcBrokerClient {
    pub fn new(config: RumqttcBrokerClientConfig) -> Self {
        Self {
            config,
            runtime: Mutex::new(None),
        }
    }

    fn build_options(&self) -> Result<MqttOptions, String> {
        let (host, port) = parse_broker_url(&self.config.broker_url)?;
        let mut options = MqttOptions::new(self.config.client_id.clone(), host, port);
        options.set_keep_alive(Duration::from_secs(self.config.keep_alive_secs.max(1)));

        if let Some(username) = &self.config.username {
            options.set_credentials(
                username.clone(),
                self.config.password.clone().unwrap_or_default(),
            );
        }

        Ok(options)
    }
}

impl fmt::Debug for RumqttcBrokerClient {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RumqttcBrokerClient")
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}

impl MqttBrokerClient for RumqttcBrokerClient {
    fn connect(
        &self,
        handler: MessageHandler,
        lifecycle_handler: LifecycleHandler,
    ) -> Result<(), String> {
        let mut runtime = self.runtime.lock().map_err(|error| error.to_string())?;
        if runtime.is_some() {
            return Ok(());
        }

        let options = self.build_options()?;
        let (client, mut connection) = Client::new(options, 32);
        lifecycle_handler(BrokerLifecycleEvent::Connecting);
        let listener = thread::spawn(move || {
            for notification in connection.iter() {
                match notification {
                    Ok(Event::Incoming(Incoming::ConnAck(_))) => {
                        lifecycle_handler(BrokerLifecycleEvent::Connected);
                    }
                    Ok(Event::Incoming(Incoming::Publish(publish))) => {
                        handler(BrokerIncomingMessage {
                            topic: publish.topic,
                            payload: publish.payload.to_vec(),
                        })
                    }
                    Ok(Event::Outgoing(Outgoing::Disconnect)) => {
                        lifecycle_handler(BrokerLifecycleEvent::Disconnected { message: None });
                    }
                    Ok(_) => {}
                    Err(error) => {
                        lifecycle_handler(BrokerLifecycleEvent::Reconnecting {
                            message: error.to_string(),
                        });
                        thread::sleep(Duration::from_millis(500));
                    }
                }
            }

            lifecycle_handler(BrokerLifecycleEvent::Disconnected {
                message: Some("mqtt listener exited".to_string()),
            });
        });

        *runtime = Some(RumqttcRuntime { client, listener });
        Ok(())
    }

    fn disconnect(&self) -> Result<(), String> {
        let mut runtime = self.runtime.lock().map_err(|error| error.to_string())?;
        if let Some(runtime) = runtime.take() {
            runtime
                .client
                .disconnect()
                .map_err(|error| error.to_string())?;
            let _ = runtime.listener.join();
        }

        Ok(())
    }

    fn subscribe(&self, topic: &str, qos: BrokerQos) -> Result<(), String> {
        let mut runtime = self.runtime.lock().map_err(|error| error.to_string())?;
        let runtime = runtime
            .as_mut()
            .ok_or_else(|| "mqtt broker client is not connected".to_string())?;

        runtime
            .client
            .subscribe(topic, qos.to_rumqttc())
            .map_err(|error| error.to_string())
    }

    fn publish(
        &self,
        topic: &str,
        qos: BrokerQos,
        retained: bool,
        payload: Vec<u8>,
    ) -> Result<(), String> {
        let mut runtime = self.runtime.lock().map_err(|error| error.to_string())?;
        let runtime = runtime
            .as_mut()
            .ok_or_else(|| "mqtt broker client is not connected".to_string())?;

        runtime
            .client
            .publish(topic, qos.to_rumqttc(), retained, payload)
            .map_err(|error| error.to_string())
    }
}

pub struct MqttControlTransport {
    config: MqttControlConfig,
    event_bus: EventBus,
    broker_client: Option<Arc<dyn MqttBrokerClient>>,
    state: Mutex<TransportState>,
}

#[derive(Debug, Default)]
struct TransportState {
    connected: bool,
    outbound_messages: Vec<String>,
    last_error: Option<String>,
}

impl MqttControlTransport {
    pub fn new(config: MqttControlConfig, event_bus: EventBus) -> Self {
        Self {
            config,
            event_bus,
            broker_client: None,
            state: Mutex::new(TransportState::default()),
        }
    }

    pub fn with_broker_client(
        config: MqttControlConfig,
        event_bus: EventBus,
        broker_client: Arc<dyn MqttBrokerClient>,
    ) -> Self {
        Self {
            config,
            event_bus,
            broker_client: Some(broker_client),
            state: Mutex::new(TransportState::default()),
        }
    }

    pub fn config(&self) -> &MqttControlConfig {
        &self.config
    }

    pub fn subscription_topics(config: &MqttControlConfig) -> Vec<String> {
        Self::subscription_specs(config)
            .into_iter()
            .map(|(topic, _)| topic)
            .collect()
    }

    pub fn queued_messages(&self) -> Result<Vec<String>, String> {
        let state = self.state.lock().map_err(|error| error.to_string())?;
        Ok(state.outbound_messages.clone())
    }

    fn publish_transport_state(
        &self,
        state: TransportConnectionState,
        message: Option<String>,
    ) -> Result<(), String> {
        self.event_bus
            .publish(RuntimeEvent::TransportStateChanged(TransportStatePayload {
                session_id: self.config.session_id.clone(),
                state,
                message,
            }))
    }

    fn subscription_specs(config: &MqttControlConfig) -> Vec<(String, BrokerQos)> {
        vec![
            (
                control_reply_topic(&config.client_id, &config.session_id),
                BrokerQos::AtLeastOnce,
            ),
            (
                events_topic(&config.client_id, &config.session_id),
                BrokerQos::AtLeastOnce,
            ),
            (
                stt_topic(&config.client_id, &config.session_id),
                BrokerQos::AtMostOnce,
            ),
            (
                summary_topic(&config.client_id, &config.session_id),
                BrokerQos::AtMostOnce,
            ),
            (
                action_items_topic(&config.client_id, &config.session_id),
                BrokerQos::AtMostOnce,
            ),
        ]
    }

    fn is_connected(&self) -> Result<bool, String> {
        let state = self.state.lock().map_err(|error| error.to_string())?;
        Ok(state.connected)
    }

    pub fn dispatch_message(event_bus: &EventBus, raw_payload: &str) -> Result<(), String> {
        let envelope: IncomingEnvelope =
            serde_json::from_str(raw_payload).map_err(|error| error.to_string())?;

        match envelope.message_type.as_str() {
            "stt_delta" | "stt_final" => {
                let payload = deserialize_payload::<IncomingTranscriptPayload>(envelope.payload)?;
                event_bus.publish(RuntimeEvent::TranscriptDelta(TranscriptDeltaPayload {
                    session_id: envelope.session_id.clone(),
                    segment_id: if payload.segment_id.is_empty() {
                        format!("{}-{}", envelope.session_id, payload.start_ms)
                    } else {
                        payload.segment_id
                    },
                    start_ms: payload.start_ms,
                    end_ms: payload.end_ms,
                    text: payload.text,
                    is_final: payload.is_final || envelope.message_type == "stt_final",
                    speaker_id: payload.speaker_id,
                    revision: payload.revision.max(1),
                }))
            }
            "summary_delta" => {
                let payload = deserialize_payload::<IncomingSummaryPayload>(envelope.payload)?;
                event_bus.publish(RuntimeEvent::SummaryDelta(SummaryDeltaPayload {
                    session_id: envelope.session_id,
                    version: payload.version.max(1),
                    updated_at: payload.updated_at,
                    abstract_text: payload.abstract_text.or(payload.text).unwrap_or_default(),
                    key_points: payload.key_points,
                    decisions: payload.decisions,
                    risks: payload.risks,
                    action_items: payload.action_items,
                    is_final: false,
                }))
            }
            "summary_final" => {
                let payload = deserialize_payload::<IncomingSummaryPayload>(envelope.payload)?;
                event_bus.publish(RuntimeEvent::SummaryDelta(SummaryDeltaPayload {
                    session_id: envelope.session_id,
                    version: payload.version.max(1),
                    updated_at: payload.updated_at,
                    abstract_text: payload.abstract_text.or(payload.text).unwrap_or_default(),
                    key_points: payload.key_points,
                    decisions: payload.decisions,
                    risks: payload.risks,
                    action_items: payload.action_items,
                    is_final: true,
                }))
            }
            "action_item_delta" | "action_item_final" => {
                let payload = deserialize_payload::<IncomingActionItemsPayload>(envelope.payload)?;
                event_bus.publish(RuntimeEvent::ActionItemsDelta(ActionItemsDeltaPayload {
                    session_id: envelope.session_id,
                    version: payload.version.max(1),
                    updated_at: payload.updated_at,
                    items: payload.items,
                    is_final: payload.is_final || envelope.message_type == "action_item_final",
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

    fn broker_message_handler(&self) -> MessageHandler {
        let event_bus = self.event_bus.clone();
        Arc::new(move |message| match String::from_utf8(message.payload) {
            Ok(payload) => {
                let _ = MqttControlTransport::dispatch_message(&event_bus, &payload);
            }
            Err(_) => {
                let _ = event_bus.publish(RuntimeEvent::TransportError {
                    message: "mqtt payload is not valid utf-8".to_string(),
                });
            }
        })
    }

    fn broker_lifecycle_handler(&self) -> LifecycleHandler {
        let event_bus = self.event_bus.clone();
        let session_id = self.config.session_id.clone();

        Arc::new(move |event| {
            let runtime_event = match event {
                BrokerLifecycleEvent::Connecting => {
                    RuntimeEvent::TransportStateChanged(TransportStatePayload {
                        session_id: session_id.clone(),
                        state: TransportConnectionState::Connecting,
                        message: None,
                    })
                }
                BrokerLifecycleEvent::Connected => {
                    RuntimeEvent::TransportStateChanged(TransportStatePayload {
                        session_id: session_id.clone(),
                        state: TransportConnectionState::Connected,
                        message: None,
                    })
                }
                BrokerLifecycleEvent::Reconnecting { message } => {
                    let _ = event_bus.publish(RuntimeEvent::TransportError {
                        message: message.clone(),
                    });
                    RuntimeEvent::TransportStateChanged(TransportStatePayload {
                        session_id: session_id.clone(),
                        state: TransportConnectionState::Reconnecting,
                        message: Some(message),
                    })
                }
                BrokerLifecycleEvent::Disconnected { message } => {
                    RuntimeEvent::TransportStateChanged(TransportStatePayload {
                        session_id: session_id.clone(),
                        state: TransportConnectionState::Disconnected,
                        message,
                    })
                }
            };

            let _ = event_bus.publish(runtime_event);
        })
    }

    pub fn start_recording(&self) -> Result<String, String> {
        self.send_empty_control_envelope(MessageType::RecordingStart, "recording-start", 2)
    }

    pub fn pause_recording(&self) -> Result<String, String> {
        self.send_empty_control_envelope(MessageType::RecordingPause, "recording-pause", 3)
    }

    pub fn resume_recording(&self) -> Result<String, String> {
        self.send_empty_control_envelope(MessageType::RecordingResume, "recording-resume", 4)
    }

    pub fn stop_recording(&self) -> Result<String, String> {
        self.send_empty_control_envelope(MessageType::RecordingStop, "recording-stop", 5)
    }

    fn send_empty_control_envelope(
        &self,
        message_type: MessageType,
        message_suffix: &str,
        seq: u64,
    ) -> Result<String, String> {
        if !self.is_connected()? {
            return Err("transport is not connected".to_string());
        }

        let message = MessageEnvelope {
            version: "v1".to_string(),
            message_id: format!("{}-{message_suffix}", self.config.session_id),
            correlation_id: None,
            client_id: self.config.client_id.clone(),
            session_id: self.config.session_id.clone(),
            seq,
            sent_at: "1970-01-01T00:00:00Z".to_string(),
            message_type,
            payload: serde_json::json!({}),
        };

        let serialized = serde_json::to_string(&message).map_err(|error| error.to_string())?;
        self.send_control_message(&serialized)?;
        Ok(serialized)
    }
}

impl fmt::Debug for MqttControlTransport {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("MqttControlTransport")
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}

impl ControlTransport for MqttControlTransport {
    fn connect(&self) -> Result<(), String> {
        self.publish_transport_state(TransportConnectionState::Connecting, None)?;

        {
            let state = self.state.lock().map_err(|error| error.to_string())?;
            if state.connected {
                return Ok(());
            }
        }

        if let Some(broker_client) = &self.broker_client {
            broker_client.connect(
                self.broker_message_handler(),
                self.broker_lifecycle_handler(),
            )?;
            for (topic, qos) in Self::subscription_specs(&self.config) {
                broker_client.subscribe(&topic, qos)?;
            }
        }

        let mut state = self.state.lock().map_err(|error| error.to_string())?;
        state.connected = true;
        if self.broker_client.is_none() {
            drop(state);
            self.publish_transport_state(TransportConnectionState::Connected, None)?;
        }
        Ok(())
    }

    fn disconnect(&self) -> Result<(), String> {
        if let Some(broker_client) = &self.broker_client {
            broker_client.disconnect()?;
        }

        let mut state = self.state.lock().map_err(|error| error.to_string())?;
        state.connected = false;
        drop(state);
        self.publish_transport_state(TransportConnectionState::Disconnected, None)?;
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
        let topic = control_topic(&self.config.client_id, &self.config.session_id);

        {
            let mut state = self.state.lock().map_err(|error| error.to_string())?;
            if !state.connected {
                return Err("transport is not connected".to_string());
            }
            state.outbound_messages.push(payload.to_string());
        }

        if let Some(broker_client) = &self.broker_client {
            broker_client.publish(
                &topic,
                BrokerQos::AtLeastOnce,
                false,
                payload.as_bytes().to_vec(),
            )?;
        }

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

fn parse_broker_url(url: &str) -> Result<(String, u16), String> {
    let trimmed = url
        .strip_prefix("tcp://")
        .or_else(|| url.strip_prefix("mqtt://"))
        .unwrap_or(url);
    let (host, port) = trimmed
        .rsplit_once(':')
        .ok_or_else(|| "mqtt broker url must include host:port".to_string())?;

    if host.trim().is_empty() {
        return Err("mqtt broker host cannot be empty".to_string());
    }

    let port = port.parse::<u16>().map_err(|error| error.to_string())?;
    Ok((host.to_string(), port))
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
    #[serde(rename = "segmentId", default)]
    segment_id: String,
    #[serde(rename = "startMs", default)]
    start_ms: u64,
    #[serde(rename = "endMs", default)]
    end_ms: u64,
    text: String,
    #[serde(rename = "isFinal", default)]
    is_final: bool,
    #[serde(rename = "speakerId", default)]
    speaker_id: Option<String>,
    #[serde(default)]
    revision: u64,
}

#[derive(Debug, Deserialize)]
struct IncomingSummaryPayload {
    #[serde(default)]
    text: Option<String>,
    #[serde(rename = "abstract", default)]
    abstract_text: Option<String>,
    #[serde(default)]
    version: u64,
    #[serde(rename = "updatedAt", default)]
    updated_at: String,
    #[serde(rename = "keyPoints", default)]
    key_points: Vec<String>,
    #[serde(default)]
    decisions: Vec<String>,
    #[serde(default)]
    risks: Vec<String>,
    #[serde(rename = "actionItems", default)]
    action_items: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct IncomingActionItemsPayload {
    #[serde(default)]
    version: u64,
    #[serde(rename = "updatedAt", default)]
    updated_at: String,
    #[serde(default)]
    items: Vec<String>,
    #[serde(rename = "isFinal", default)]
    is_final: bool,
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
    use std::sync::{Arc, Mutex};

    use crate::events::bus::EventBus;
    use crate::events::types::{RuntimeEvent, TransportConnectionState, TransportStatePayload};
    use crate::transport::control_transport::ControlTransport;

    use super::{
        BrokerIncomingMessage, BrokerLifecycleEvent, BrokerQos, LifecycleHandler, MessageHandler,
        MqttBrokerClient, MqttControlConfig, MqttControlTransport,
    };

    #[derive(Default)]
    struct FakeBrokerClientState {
        connect_calls: usize,
        disconnect_calls: usize,
        subscriptions: Vec<(String, BrokerQos)>,
        publishes: Vec<(String, BrokerQos, bool, Vec<u8>)>,
        handler: Option<MessageHandler>,
        lifecycle_handler: Option<LifecycleHandler>,
    }

    #[derive(Clone, Default)]
    struct FakeBrokerClient {
        state: Arc<Mutex<FakeBrokerClientState>>,
    }

    impl FakeBrokerClient {
        fn snapshot(&self) -> FakeBrokerClientState {
            let state = self.state.lock().unwrap();
            FakeBrokerClientState {
                connect_calls: state.connect_calls,
                disconnect_calls: state.disconnect_calls,
                subscriptions: state.subscriptions.clone(),
                publishes: state.publishes.clone(),
                handler: state.handler.clone(),
                lifecycle_handler: state.lifecycle_handler.clone(),
            }
        }

        fn push_incoming(&self, topic: &str, payload: &str) {
            let handler = self.state.lock().unwrap().handler.clone().unwrap();
            handler(BrokerIncomingMessage {
                topic: topic.to_string(),
                payload: payload.as_bytes().to_vec(),
            });
        }

        fn push_lifecycle(&self, event: BrokerLifecycleEvent) {
            let handler = self
                .state
                .lock()
                .unwrap()
                .lifecycle_handler
                .clone()
                .unwrap();
            handler(event);
        }
    }

    impl MqttBrokerClient for FakeBrokerClient {
        fn connect(
            &self,
            handler: MessageHandler,
            lifecycle_handler: LifecycleHandler,
        ) -> Result<(), String> {
            let mut state = self.state.lock().map_err(|error| error.to_string())?;
            state.connect_calls += 1;
            state.handler = Some(handler);
            state.lifecycle_handler = Some(lifecycle_handler);
            Ok(())
        }

        fn disconnect(&self) -> Result<(), String> {
            let mut state = self.state.lock().map_err(|error| error.to_string())?;
            state.disconnect_calls += 1;
            Ok(())
        }

        fn subscribe(&self, topic: &str, qos: BrokerQos) -> Result<(), String> {
            let mut state = self.state.lock().map_err(|error| error.to_string())?;
            state.subscriptions.push((topic.to_string(), qos));
            Ok(())
        }

        fn publish(
            &self,
            topic: &str,
            qos: BrokerQos,
            retained: bool,
            payload: Vec<u8>,
        ) -> Result<(), String> {
            let mut state = self.state.lock().map_err(|error| error.to_string())?;
            state
                .publishes
                .push((topic.to_string(), qos, retained, payload));
            Ok(())
        }
    }

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
                    "segmentId": "segment-1",
                    "startMs": 0,
                    "endMs": 1200,
                    "text": "这是新的转写片段",
                    "isFinal": false,
                    "revision": 1
                }
            }"#,
        )
        .unwrap();

        let events = event_bus.snapshot().unwrap();
        assert_eq!(
            events,
            vec![RuntimeEvent::TranscriptDelta(
                crate::events::types::TranscriptDeltaPayload {
                    session_id: "session-1".to_string(),
                    segment_id: "segment-1".to_string(),
                    start_ms: 0,
                    end_ms: 1_200,
                    text: "这是新的转写片段".to_string(),
                    is_final: false,
                    speaker_id: None,
                    revision: 1,
                }
            )]
        );
    }

    #[test]
    fn open_session_serializes_hello_envelope_and_queues_it() {
        let transport = MqttControlTransport::new(
            MqttControlConfig {
                client_id: "client-a".to_string(),
                session_id: "session-1".to_string(),
            },
            EventBus::default(),
        );

        transport.connect().unwrap();
        let hello = transport.open_session("架构评审会").unwrap();

        let events = transport.event_bus.snapshot().unwrap();

        assert!(hello.contains("\"type\":\"session/hello\""));
        assert!(hello.contains("\"title\":\"架构评审会\""));
        assert_eq!(transport.queued_messages().unwrap(), vec![hello]);
        assert_eq!(
            events,
            vec![
                RuntimeEvent::TransportStateChanged(TransportStatePayload {
                    session_id: "session-1".to_string(),
                    state: TransportConnectionState::Connecting,
                    message: None,
                }),
                RuntimeEvent::TransportStateChanged(TransportStatePayload {
                    session_id: "session-1".to_string(),
                    state: TransportConnectionState::Connected,
                    message: None,
                }),
            ]
        );
    }

    #[test]
    fn on_error_emits_transport_error_event() {
        let transport = MqttControlTransport::new(
            MqttControlConfig {
                client_id: "client-a".to_string(),
                session_id: "session-1".to_string(),
            },
            EventBus::default(),
        );
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

    #[test]
    fn dispatch_message_emits_structured_summary_delta_event() {
        let event_bus = EventBus::default();

        MqttControlTransport::dispatch_message(
            &event_bus,
            r#"{
                "type": "summary_delta",
                "sessionId": "session-1",
                "payload": {
                    "version": 2,
                    "updatedAt": "2026-04-22T10:00:00Z",
                    "abstract": "会议纪要持续刷新中",
                    "keyPoints": ["双路采集保留原始 WAV"],
                    "decisions": ["首版控制通道使用 MQTT"],
                    "risks": ["需要补断点续传"],
                    "actionItems": ["继续联调 summary_final"]
                }
            }"#,
        )
        .unwrap();

        assert_eq!(
            event_bus.snapshot().unwrap(),
            vec![RuntimeEvent::SummaryDelta(
                crate::events::types::SummaryDeltaPayload {
                    session_id: "session-1".to_string(),
                    version: 2,
                    updated_at: "2026-04-22T10:00:00Z".to_string(),
                    abstract_text: "会议纪要持续刷新中".to_string(),
                    key_points: vec!["双路采集保留原始 WAV".to_string()],
                    decisions: vec!["首版控制通道使用 MQTT".to_string()],
                    risks: vec!["需要补断点续传".to_string()],
                    action_items: vec!["继续联调 summary_final".to_string()],
                    is_final: false,
                }
            )]
        );
    }

    #[test]
    fn dispatch_message_emits_action_items_delta_event() {
        let event_bus = EventBus::default();

        MqttControlTransport::dispatch_message(
            &event_bus,
            r#"{
                "type": "action_item_delta",
                "sessionId": "session-1",
                "payload": {
                    "version": 4,
                    "updatedAt": "2026-04-22T10:02:00Z",
                    "items": ["跟进会议结论同步"],
                    "isFinal": false
                }
            }"#,
        )
        .unwrap();

        assert_eq!(
            event_bus.snapshot().unwrap(),
            vec![RuntimeEvent::ActionItemsDelta(
                crate::events::types::ActionItemsDeltaPayload {
                    session_id: "session-1".to_string(),
                    version: 4,
                    updated_at: "2026-04-22T10:02:00Z".to_string(),
                    items: vec!["跟进会议结论同步".to_string()],
                    is_final: false,
                }
            )]
        );
    }

    #[test]
    fn broker_backed_connect_subscribes_topics_and_dispatches_transcript_delta() {
        let event_bus = EventBus::default();
        let broker = FakeBrokerClient::default();
        let transport = MqttControlTransport::with_broker_client(
            MqttControlConfig {
                client_id: "client-a".to_string(),
                session_id: "session-1".to_string(),
            },
            event_bus.clone(),
            Arc::new(broker.clone()),
        );

        transport.connect().unwrap();
        broker.push_lifecycle(BrokerLifecycleEvent::Connected);
        broker.push_incoming(
            "meetings/client-a/session/session-1/stt",
            r#"{
                "type": "stt_delta",
                "sessionId": "session-1",
                "payload": {
                    "segmentId": "segment-live-1",
                    "startMs": 1200,
                    "endMs": 2400,
                    "text": "broker 推送的实时转写",
                    "isFinal": false,
                    "revision": 3
                }
            }"#,
        );

        let snapshot = broker.snapshot();
        assert_eq!(snapshot.connect_calls, 1);
        assert_eq!(
            snapshot.subscriptions,
            vec![
                (
                    "meetings/client-a/session/session-1/control/reply".to_string(),
                    BrokerQos::AtLeastOnce,
                ),
                (
                    "meetings/client-a/session/session-1/events".to_string(),
                    BrokerQos::AtLeastOnce,
                ),
                (
                    "meetings/client-a/session/session-1/stt".to_string(),
                    BrokerQos::AtMostOnce,
                ),
                (
                    "meetings/client-a/session/session-1/summary".to_string(),
                    BrokerQos::AtMostOnce,
                ),
                (
                    "meetings/client-a/session/session-1/action-items".to_string(),
                    BrokerQos::AtMostOnce,
                ),
            ]
        );
        assert_eq!(
            event_bus.snapshot().unwrap(),
            vec![
                RuntimeEvent::TransportStateChanged(TransportStatePayload {
                    session_id: "session-1".to_string(),
                    state: TransportConnectionState::Connecting,
                    message: None,
                }),
                RuntimeEvent::TransportStateChanged(TransportStatePayload {
                    session_id: "session-1".to_string(),
                    state: TransportConnectionState::Connected,
                    message: None,
                }),
                RuntimeEvent::TranscriptDelta(crate::events::types::TranscriptDeltaPayload {
                    session_id: "session-1".to_string(),
                    segment_id: "segment-live-1".to_string(),
                    start_ms: 1_200,
                    end_ms: 2_400,
                    text: "broker 推送的实时转写".to_string(),
                    is_final: false,
                    speaker_id: None,
                    revision: 3,
                })
            ]
        );
    }

    #[test]
    fn broker_backed_open_session_publishes_hello_to_control_topic() {
        let broker = FakeBrokerClient::default();
        let transport = MqttControlTransport::with_broker_client(
            MqttControlConfig {
                client_id: "client-a".to_string(),
                session_id: "session-1".to_string(),
            },
            EventBus::default(),
            Arc::new(broker.clone()),
        );

        transport.connect().unwrap();
        let hello = transport.open_session("架构评审会").unwrap();

        let snapshot = broker.snapshot();
        assert_eq!(snapshot.publishes.len(), 1);
        assert_eq!(
            snapshot.publishes[0].0,
            "meetings/client-a/session/session-1/control".to_string()
        );
        assert_eq!(snapshot.publishes[0].1, BrokerQos::AtLeastOnce);
        assert!(!snapshot.publishes[0].2);
        assert_eq!(
            String::from_utf8(snapshot.publishes[0].3.clone()).unwrap(),
            hello
        );
    }

    #[test]
    fn broker_backed_start_recording_publishes_start_to_control_topic() {
        let broker = FakeBrokerClient::default();
        let transport = MqttControlTransport::with_broker_client(
            MqttControlConfig {
                client_id: "client-a".to_string(),
                session_id: "session-1".to_string(),
            },
            EventBus::default(),
            Arc::new(broker.clone()),
        );

        transport.connect().unwrap();
        let command = transport.start_recording().unwrap();

        let snapshot = broker.snapshot();
        assert_eq!(snapshot.publishes.len(), 1);
        assert_eq!(
            snapshot.publishes[0].0,
            "meetings/client-a/session/session-1/control".to_string()
        );
        assert!(command.contains("\"type\":\"recording/start\""));
        assert_eq!(
            String::from_utf8(snapshot.publishes[0].3.clone()).unwrap(),
            command
        );
    }

    #[test]
    fn broker_backed_stop_recording_publishes_stop_to_control_topic() {
        let broker = FakeBrokerClient::default();
        let transport = MqttControlTransport::with_broker_client(
            MqttControlConfig {
                client_id: "client-a".to_string(),
                session_id: "session-9".to_string(),
            },
            EventBus::default(),
            Arc::new(broker.clone()),
        );

        transport.connect().unwrap();
        let command = transport.stop_recording().unwrap();

        let snapshot = broker.snapshot();
        assert_eq!(snapshot.publishes.len(), 1);
        assert_eq!(
            snapshot.publishes[0].0,
            "meetings/client-a/session/session-9/control".to_string()
        );
        assert!(command.contains("\"type\":\"recording/stop\""));
        assert_eq!(
            String::from_utf8(snapshot.publishes[0].3.clone()).unwrap(),
            command
        );
    }

    #[test]
    fn broker_lifecycle_reconnects_are_forwarded_to_runtime_events() {
        let event_bus = EventBus::default();
        let broker = FakeBrokerClient::default();
        let transport = MqttControlTransport::with_broker_client(
            MqttControlConfig {
                client_id: "client-a".to_string(),
                session_id: "session-9".to_string(),
            },
            event_bus.clone(),
            Arc::new(broker.clone()),
        );

        transport.connect().unwrap();
        broker.push_lifecycle(BrokerLifecycleEvent::Reconnecting {
            message: "connection reset".to_string(),
        });

        assert_eq!(
            event_bus.snapshot().unwrap(),
            vec![
                RuntimeEvent::TransportStateChanged(TransportStatePayload {
                    session_id: "session-9".to_string(),
                    state: TransportConnectionState::Connecting,
                    message: None,
                }),
                RuntimeEvent::TransportError {
                    message: "connection reset".to_string(),
                },
                RuntimeEvent::TransportStateChanged(TransportStatePayload {
                    session_id: "session-9".to_string(),
                    state: TransportConnectionState::Reconnecting,
                    message: Some("connection reset".to_string()),
                }),
            ]
        );
    }
}
