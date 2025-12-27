use lsp_message::LspMessage;
use std::io::BufReader;
#[cfg(target_family = "unix")]
use std::os::unix::net::UnixStream;
#[cfg(target_family = "unix")]
use std::{io, sync::Mutex};

pub struct Connection {
    #[cfg(target_family = "unix")]
    pub stream: Mutex<UnixStream>,
}

impl Connection {
    pub fn wait_for_message(&self) -> io::Result<Option<LspMessage>> {
        let stream = self.stream.lock().unwrap();
        LspMessage::read(&mut BufReader::new(&*stream))
    }

    pub fn send(&self, message: LspMessage) -> io::Result<()> {
        let stream = self.stream.lock().unwrap();
        message.write(&mut &*stream)
    }
}
