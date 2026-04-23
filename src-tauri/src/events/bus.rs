use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};

use crate::events::types::RuntimeEvent;

#[derive(Clone, Default)]
pub struct EventBus {
    events: Arc<Mutex<Vec<RuntimeEvent>>>,
    subscribers: Arc<Mutex<Vec<Sender<RuntimeEvent>>>>,
}

impl EventBus {
    pub fn publish(&self, event: RuntimeEvent) -> Result<(), String> {
        {
            let mut events = self.events.lock().map_err(|error| error.to_string())?;
            events.push(event.clone());
        }

        let mut subscribers = self.subscribers.lock().map_err(|error| error.to_string())?;
        subscribers.retain(|sender| sender.send(event.clone()).is_ok());
        Ok(())
    }

    pub fn subscribe(&self) -> Result<Receiver<RuntimeEvent>, String> {
        let (sender, receiver) = channel();
        let mut subscribers = self.subscribers.lock().map_err(|error| error.to_string())?;
        subscribers.push(sender);
        Ok(receiver)
    }

    #[cfg(test)]
    pub fn snapshot(&self) -> Result<Vec<RuntimeEvent>, String> {
        let events = self.events.lock().map_err(|error| error.to_string())?;
        Ok(events.clone())
    }
}

#[cfg(test)]
mod tests {
    use crate::events::types::RuntimeEvent;

    use super::EventBus;

    #[test]
    fn subscribers_receive_published_runtime_events() {
        let bus = EventBus::default();
        let receiver = bus.subscribe().unwrap();

        bus.publish(RuntimeEvent::Heartbeat {
            session_id: "session-1".to_string(),
        })
        .unwrap();

        assert_eq!(
            receiver.recv().unwrap(),
            RuntimeEvent::Heartbeat {
                session_id: "session-1".to_string(),
            }
        );
    }
}
