use std::{fs::File, io::Write, path::Path};

pub struct Logger {
    file: Option<File>,
}

impl Logger {
    pub fn new_with_file(filename: impl AsRef<Path>) -> Result<Self, ()> {
        let file = File::create(filename).map_err(|_| ())?;
        Ok(Self { file: Some(file) })
    }
}

impl Write for Logger {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self.file.as_mut() {
            Some(file) => file.write(buf),
            None => Ok(buf.len()),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self.file.as_mut() {
            Some(file) => file.flush(),
            None => Ok(()),
        }
    }
}
