use crate::{Artifact, Control, ReqState, Rt, Thrd, WakeAfter};
use diagnostics::ErrorDiagnostic;

type Search = ();

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub enum Req {
    BuildExecutable,
    RootFile,
    Test,
    Search(Search),
}

impl Req {
    pub fn poll(&self, thread: &mut impl Thrd, state: &mut ReqState) -> Result<(), Control> {
        match self {
            Req::RootFile => unreachable!("input value"),
            Req::Test => {
                let root_file = thread.demand(Req::RootFile)?;
                *state = ReqState::Complete(root_file.clone());
                return Ok(());
            }
            Req::BuildExecutable => {
                let test = thread.demand(Req::Test)?;
                *state = ReqState::Complete(test.clone());
                return Ok(());
            }
            Req::Search(_) => {
                *state = ReqState::Complete(Artifact::Void);
                return Ok(());
            }
        }
    }
}
