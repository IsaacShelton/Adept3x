use crate::Queue;
use lsp_message::LspMessage;
#[cfg(target_family = "unix")]
use std::os::unix::net::UnixListener;
use std::{
    io::{self, BufReader},
    os::unix::net::UnixStream,
    sync::atomic::{AtomicBool, Ordering},
};

pub struct Daemon {
    #[cfg(target_family = "unix")]
    pub listener: UnixListener,

    pub should_exit: AtomicBool,
    pub queue: Queue,
}

impl Daemon {
    #[cfg(target_family = "unix")]
    pub fn new(listener: UnixListener) -> Self {
        Self {
            listener,
            should_exit: false.into(),
            queue: Queue::default(),
        }
    }

    pub fn wait_for_message(&self, client_stream: &UnixStream) -> io::Result<Option<LspMessage>> {
        LspMessage::read(&mut BufReader::new(client_stream))
    }

    pub fn send(message: LspMessage, mut client_stream: &UnixStream) -> io::Result<()> {
        message.write(&mut client_stream)
    }

    pub fn intend_exit(&self) {
        self.should_exit.store(true, Ordering::SeqCst);
    }

    pub fn should_exit(&self) -> bool {
        self.should_exit.load(Ordering::SeqCst)
    }
}
