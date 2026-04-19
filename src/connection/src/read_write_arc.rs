use std::sync::Arc;

pub struct ReadWriteArc<T> {
    value: Arc<T>,
}

impl<T> ReadWriteArc<T> {
    pub fn new(value: T) -> Self {
        Self {
            value: Arc::new(value),
        }
    }

    pub fn dupe(&self) -> Self {
        Self {
            value: Arc::clone(&self.value),
        }
    }

    pub fn as_ref(&self) -> &T {
        self.value.as_ref()
    }
}

impl<T: std::io::Read> std::io::Read for ReadWriteArc<T>
where
    for<'a> &'a T: std::io::Read,
{
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.value.as_ref().read(buf)
    }
}

impl<T: std::io::Write> std::io::Write for ReadWriteArc<T>
where
    for<'a> &'a T: std::io::Write,
{
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.value.as_ref().write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.value.as_ref().flush()
    }
}
