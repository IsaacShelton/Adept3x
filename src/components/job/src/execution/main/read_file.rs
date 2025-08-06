use crate::{
    Continuation, Execution, ExecutionCtx, Executor,
    io::{IoRequest, IoRequestHandle},
    sub_task::SubTask,
};
use diagnostics::ErrorDiagnostic;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub enum ReadFile {
    NotStarted(Option<PathBuf>),
    Pending(IoRequestHandle),
    Complete(Result<String, String>),
}

impl ReadFile {
    pub fn new(filename: PathBuf) -> Self {
        Self::NotStarted(Some(filename))
    }

    fn unwrap_complete(&self) -> Result<&str, &str> {
        match self {
            ReadFile::Complete(result) => result
                .as_ref()
                .map(|ok| ok.as_str())
                .map_err(|err| err.as_str()),
            _ => panic!("expected ReadFile to be complete"),
        }
    }
}

impl<'env> SubTask<'env> for ReadFile {
    type SubArtifact<'a>
        = Result<&'a str, &'a str>
    where
        Self: 'a,
        'env: 'a;

    type UserData<'a>
        = ()
    where
        Self: 'a,
        'env: 'a;

    fn execute_sub_task<'a, 'ctx>(
        &'a mut self,
        executor: &'a Executor<'env>,
        ctx: &'ctx mut ExecutionCtx<'env>,
        _user_data: Self::UserData<'a>,
    ) -> Result<
        Self::SubArtifact<'a>,
        Result<impl Fn(Execution<'env>) -> Continuation<'env> + 'static, ErrorDiagnostic>,
    > {
        match self {
            ReadFile::NotStarted(path_buf) => {
                *self = Self::Pending(executor.request_io(
                    IoRequest::ReadFile(path_buf.take().unwrap()),
                    ctx.self_task(),
                ));

                return Err(Ok(Continuation::PendingIo));
            }
            ReadFile::Pending(io_handle) => {
                *self = Self::Complete(
                    executor
                        .completed_io
                        .lock()
                        .unwrap()
                        .remove(&io_handle)
                        .unwrap()
                        .unwrap_fulfilled()
                        .payload,
                );
                return Ok(self.unwrap_complete());
            }
            ReadFile::Complete(_) => {
                return Ok(self.unwrap_complete());
            }
        }
    }
}
