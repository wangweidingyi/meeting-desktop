pub trait ControlTransport {
    fn connect(&self) -> Result<(), String>;
    fn disconnect(&self) -> Result<(), String>;
    fn open_session(&self) -> Result<(), String>;
    fn close_session(&self) -> Result<(), String>;
}
