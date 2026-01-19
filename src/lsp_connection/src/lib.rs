mod ide_connection;

pub use crate::ide_connection::IdeConnection;
use crate::ide_connection::IdeConnectionSender;
use std::sync::Arc;

pub struct LspConnection {
    pub state: LspConnectionState,
    pub ide: IdeConnection,
    pub ide_sender: IdeConnectionSender,
    pub daemon: Arc<daemon::Connection>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum LspConnectionState {
    Started,
    Initialized,
    Shutdown,
}

impl LspConnection {
    pub fn new(
        ide: IdeConnection,
        ide_sender: IdeConnectionSender,
        daemon: daemon::Connection,
    ) -> Self {
        Self {
            state: LspConnectionState::Started,
            ide,
            ide_sender,
            daemon: Arc::new(daemon),
        }
    }
}
