mod read_write_arc;

use crate::read_write_arc::ReadWriteArc;
#[cfg(target_family = "unix")]
use std::{
    io::BufReader,
    os::unix::net::UnixStream,
    sync::{Arc, Mutex},
};
use std::{path::Path, time::Duration};

pub struct Connection {
    #[cfg(target_family = "unix")]
    stream: ReadWriteArc<UnixStream>,
    #[cfg(target_family = "unix")]
    writer: Arc<Mutex<()>>,
    #[cfg(target_family = "unix")]
    reader: Arc<Mutex<BufReader<ReadWriteArc<UnixStream>>>>,
}

impl Connection {
    #[cfg(target_family = "unix")]
    pub fn new_unix(stream: UnixStream) -> Self {
        let stream = ReadWriteArc::new(stream);

        Self {
            stream: stream.dupe(),
            writer: Arc::new(Mutex::new(())),
            reader: Arc::new(Mutex::new(BufReader::new(stream))),
        }
    }

    pub fn dupe(&self) -> Self {
        #[cfg(target_family = "unix")]
        {
            Self {
                stream: self.stream.dupe(),
                reader: Arc::clone(&self.reader),
                writer: Arc::clone(&self.writer),
            }
        }

        #[cfg(target_family = "windows")]
        {
            panic!("Connection::dupe - Windows is not supported")
        }
    }

    pub fn connect(filepath: &Path) -> Result<Self, ()> {
        #[cfg(target_family = "unix")]
        {
            UnixStream::connect(filepath)
                .map(|stream| Self::new_unix(stream))
                .map_err(|_| ())
        }

        #[cfg(target_family = "windows")]
        {
            let _ = filepath;
            panic!("Connection::connect - Windows is not supported")
        }
    }

    pub fn with_writer<Z>(&self, f: impl Fn(&mut dyn std::io::Write) -> Z) -> Option<Z> {
        #[cfg(target_family = "unix")]
        {
            if let Ok(_) = self.writer.lock() {
                return Some(f(&mut self.stream.as_ref()));
            } else {
                return None;
            }
        }

        #[cfg(target_family = "windows")]
        {
            panic!("Connection::with_writer - Windows is not supported")
        }
    }

    pub fn with_reader<Z>(&self, f: impl Fn(&mut dyn std::io::BufRead) -> Z) -> Option<Z> {
        #[cfg(target_family = "unix")]
        {
            use std::ops::DerefMut;

            if let Ok(mut reader) = self.reader.lock() {
                return Some(f(reader.deref_mut()));
            } else {
                return None;
            }
        }

        #[cfg(target_family = "windows")]
        {
            panic!("Connection::with_reader - Windows is not supported")
        }
    }

    pub fn set_non_blocking(&self, non_blocking: bool, timeout: Option<Duration>) {
        #[cfg(target_family = "unix")]
        {
            self.stream.as_ref().set_nonblocking(non_blocking).unwrap();
            self.stream.as_ref().set_read_timeout(timeout).unwrap();
        }

        #[cfg(target_family = "windows")]
        {
            panic!("Connection::set_non_blocking - Windows is not supported")
        }
    }
}
