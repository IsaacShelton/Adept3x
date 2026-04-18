use lsp_message::LspMessage;
#[cfg(target_family = "unix")]
use std::os::unix::net::UnixStream;
use std::{io, path::Path};

pub struct Connection {
    #[cfg(target_family = "unix")]
    pub stream: UnixStream,
}

impl Connection {
    pub fn send(&self, message: LspMessage) -> io::Result<()> {
        #[cfg(target_family = "unix")]
        {
            message.write(&mut &self.stream)
        }

        #[cfg(target_family = "windows")]
        {
            let _ = message;
            panic!("Connection::send - Windows is not supported")
        }
    }

    pub fn connect(filepath: &Path) -> Result<Self, ()> {
        #[cfg(target_family = "unix")]
        {
            UnixStream::connect(filepath)
                .map(|stream| Self { stream })
                .map_err(|_| ())
        }

        #[cfg(target_family = "windows")]
        {
            let _ = filepath;
            panic!("Connection::connect - Windows is not supported")
        }
    }
}
