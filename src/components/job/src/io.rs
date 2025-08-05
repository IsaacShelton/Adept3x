use crate::TaskRef;
use derive_more::Unwrap;
use std::path::PathBuf;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
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

#[derive(Debug, Default, Unwrap)]
pub enum IoRequestStatus<'env> {
    PendingThen(TaskRef<'env>),
    Fulfilled(IoResponse),
    #[default]
    Consumed,
}
