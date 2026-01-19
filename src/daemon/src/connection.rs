use lsp_message::LspMessage;
use std::io;
#[cfg(target_family = "unix")]
use std::os::unix::net::UnixStream;

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
            panic!("Windows is not supported")
        }
    }
}
