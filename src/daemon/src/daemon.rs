use connection::Connection;
use idle_tracker::IdleTracker;
use lsp_message::LspMessage;
use request::PfIn;
use rt_st_in::{RtStIn, RtStInQuery};
#[cfg(target_family = "unix")]
use std::os::unix::net::UnixListener;
use std::{collections::VecDeque, io, sync::Mutex};

pub struct Daemon {
    #[cfg(target_family = "unix")]
    pub listener: UnixListener,
    pub idle_tracker: IdleTracker,
    pub rt: Mutex<RtStIn<'static, PfIn>>,
    pub queries: Mutex<VecDeque<RtStInQuery<'static, PfIn>>>,
}

impl Daemon {
    #[cfg(target_family = "unix")]
    pub fn new(listener: UnixListener) -> Self {
        use rt_st_in::ReqCache;
        use std::time::Duration;

        Self {
            listener,
            idle_tracker: IdleTracker::new(Duration::from_secs(5)),
            rt: Mutex::new(RtStIn::new(ReqCache::default())),
            queries: Mutex::new(VecDeque::default()),
        }
    }

    pub fn wait_for_message(&self, connection: &Connection) -> io::Result<Option<LspMessage>> {
        LspMessage::recv(connection)
    }

    pub fn send(message: LspMessage, connection: &Connection) -> io::Result<()> {
        LspMessage::send(connection, message)
    }

    pub fn should_exit(&self) -> bool {
        self.idle_tracker.should_shutdown()
    }
}
