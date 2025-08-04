use std::path::PathBuf;

#[derive(Copy, Clone, Debug)]
pub struct IoRequestHandle(pub usize);

#[derive(Debug)]
pub struct IoResponse {
    pub handle: IoRequestHandle,
    pub payload: Result<String, String>,
}

#[derive(Debug)]
pub enum IoRequest {
    ReadFile(PathBuf),
}
