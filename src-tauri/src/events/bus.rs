use std::sync::Mutex;

use crate::events::types::RuntimeEvent;

#[derive(Default)]
pub struct EventBus {
    events: Mutex<Vec<RuntimeEvent>>,
}

impl EventBus {
    pub fn publish(&self, event: RuntimeEvent) -> Result<(), String> {
        let mut events = self.events.lock().map_err(|error| error.to_string())?;
        events.push(event);
        Ok(())
    }
}
