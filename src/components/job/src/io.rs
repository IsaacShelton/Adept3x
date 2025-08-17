use std::path::PathBuf;

#[derive(Debug)]
pub struct IoResponse {
    pub payload: Result<String, String>,
}

#[derive(Debug)]
pub enum IoRequest {
    ReadFile(PathBuf),
}
