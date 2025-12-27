use lsp_message::LspMessage;
#[cfg(target_family = "unix")]
use std::os::unix::net::UnixStream;
#[cfg(target_family = "unix")]
use std::sync::Mutex;
use std::{io, io::BufReader};

pub struct Connection {
    #[cfg(target_family = "unix")]
    pub stream: Mutex<UnixStream>,
}

impl Connection {
    pub fn wait_for_message(&self) -> io::Result<Option<LspMessage>> {
        #[cfg(target_family = "unix")]
        {
            let stream = self.stream.lock().unwrap();
            LspMessage::read(&mut BufReader::new(&*stream))
        }

        #[cfg(target_family = "windows")]
        panic!("Windows is not supported")
    }

    pub fn send(&self, message: LspMessage) -> io::Result<()> {
        #[cfg(target_family = "unix")]
        {
            let stream = self.stream.lock().unwrap();
            message.write(&mut &*stream)
        }

        #[cfg(target_family = "windows")]
        panic!("Windows is not supported")
    }
}
