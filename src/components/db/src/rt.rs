use crate::{
    Artifact, BlockOn, NumberedRevision, Req, ReqState, WakeAfter, timeout::ShouldUnblock,
};
use derive_more::From;
use diagnostics::ErrorDiagnostic;
use std::time::{Duration, Instant};

pub trait Rt {
    type Revision;
    type StringKey;

    /// How to set/change an input value
    fn set_input(&mut self, req: Req, rev: Self::Revision, value: Artifact);

    /// How to loop
    fn block_on(
        &mut self,
        req: Req,
        timeout: impl ShouldUnblock,
    ) -> Result<BlockOn<&Artifact>, ErrorDiagnostic>;

    /// How to load allocated strings
    fn read_str<F, Ret>(&self, string_key: Self::StringKey, f: F) -> Ret
    where
        F: FnMut(&str) -> Ret;
}

pub trait Thrd {
    type Runtime: Rt;

    /// How to allocate strings
    fn alloc_str(&mut self, content: &str) -> <Self::Runtime as Rt>::StringKey;

    /// Get the runtime this thread is running on behalf of
    fn runtime(&self) -> &Self::Runtime;

    fn demand(&mut self, req: Req) -> Result<&Artifact, MustSuspend>;
}

#[derive(From)]
pub enum Control {
    Suspend,
    Error(ErrorDiagnostic),
}

pub struct MustSuspend;

impl From<MustSuspend> for Control {
    fn from(value: MustSuspend) -> Self {
        Self::Suspend
    }
}
