use idle_tracker::IdleTracker;
use lsp_message::LspMessage;
use request::{PfIn, RtStIn};
#[cfg(target_family = "unix")]
use std::os::unix::net::UnixListener;
use std::{
    io::{self, BufReader, Read, Write},
    sync::Mutex,
};

pub struct Daemon {
    #[cfg(target_family = "unix")]
    pub listener: UnixListener,
    pub idle_tracker: IdleTracker,
    pub rt: Mutex<RtStIn<'static, PfIn>>,
}

impl Daemon {
    #[cfg(target_family = "unix")]
    pub fn new(listener: UnixListener) -> Self {
        use request::ReqCache;
        use std::time::Duration;

        Self {
            listener,
            idle_tracker: IdleTracker::new(Duration::from_secs(5)),
            rt: Mutex::new(RtStIn::new(ReqCache::default())),
        }
    }

    pub fn wait_for_message(&self, client_stream: impl Read) -> io::Result<Option<LspMessage>> {
        LspMessage::read(&mut BufReader::new(client_stream))
    }

    pub fn send(message: LspMessage, mut client_stream: impl Write) -> io::Result<()> {
        message.write(&mut client_stream)
    }

    pub fn should_exit(&self) -> bool {
        self.idle_tracker.should_shutdown()
    }
}
