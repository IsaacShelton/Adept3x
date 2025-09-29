use crate::{Continuation, Execution, ExecutionCtx, Executor, io::IoRequest, sub_task::SubTask};
use diagnostics::ErrorDiagnostic;
use std::path::Path;

#[derive(Clone, Debug)]
pub enum ReadFile<'env> {
    NotStarted(&'env Path),
    Pending,
    Complete(Result<String, String>),
}

impl<'env> ReadFile<'env> {
    pub fn new(filename: &'env Path) -> Self {
        Self::NotStarted(filename)
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

impl<'env> SubTask<'env> for ReadFile<'env> {
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
        _executor: &'a Executor<'env>,
        ctx: &'ctx mut ExecutionCtx<'env>,
        _user_data: Self::UserData<'a>,
    ) -> Result<
        Self::SubArtifact<'a>,
        Result<impl FnOnce(Execution<'env>) -> Continuation<'env> + 'env, ErrorDiagnostic>,
    > {
        match self {
            ReadFile::NotStarted(path) => {
                let path_buf = path.to_path_buf();
                *self = Self::Pending;
                return Err(Ok(move |executable| {
                    Continuation::RequestIo(executable, IoRequest::ReadFile(path_buf))
                }));
            }
            ReadFile::Pending => {
                *self = Self::Complete(ctx.io_response().unwrap().payload);
                return Ok(self.unwrap_complete());
            }
            ReadFile::Complete(_) => {
                return Ok(self.unwrap_complete());
            }
        }
    }
}
