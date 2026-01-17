use crate::Queue;
use idle_tracker::IdleTracker;
use lsp_message::LspMessage;
use std::io::{self, BufReader};
#[cfg(target_family = "unix")]
use std::os::unix::net::UnixListener;
#[cfg(target_family = "unix")]
use std::os::unix::net::UnixStream;

pub struct Daemon {
    #[cfg(target_family = "unix")]
    pub listener: UnixListener,
    pub idle_tracker: IdleTracker,
    pub queue: Queue,
}

impl Daemon {
    #[cfg(target_family = "unix")]
    pub fn new(listener: UnixListener) -> Self {
        use std::time::Duration;

        Self {
            listener,
            queue: Queue::default(),
            idle_tracker: IdleTracker::new(Duration::from_secs(5)),
        }
    }

    #[cfg(target_family = "unix")]
    pub fn wait_for_message(&self, client_stream: &UnixStream) -> io::Result<Option<LspMessage>> {
        LspMessage::read(&mut BufReader::new(client_stream))
    }

    #[cfg(target_family = "unix")]
    pub fn send(message: LspMessage, mut client_stream: &UnixStream) -> io::Result<()> {
        message.write(&mut client_stream)
    }

    pub fn should_exit(&self) -> bool {
        self.idle_tracker.should_shutdown()
    }
}
