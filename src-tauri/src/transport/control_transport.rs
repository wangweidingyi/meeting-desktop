use crate::events::bus::EventBus;

pub trait ControlTransport {
    fn connect(&self) -> Result<(), String>;
    fn disconnect(&self) -> Result<(), String>;
    fn open_session(&self, title: &str) -> Result<String, String>;
    fn close_session(&self) -> Result<String, String>;
    fn send_control_message(&self, payload: &str) -> Result<(), String>;
    fn on_message(&self, event_bus: &EventBus, payload: &str) -> Result<(), String>;
    fn on_error(&self, event_bus: &EventBus, message: &str) -> Result<(), String>;
}
