#![allow(unused)]

mod artifact;
mod block_on;
mod db;
mod non_zero;
mod req;
mod req_info;
mod req_state;
mod rev;
mod rt;
mod rt_st_incr;
mod timeout;
mod wake_after;

use crate::timeout::{ShouldUnblock, TimeoutAfterSteps, TimeoutAt, TimeoutNever};
pub use artifact::*;
pub use block_on::*;
pub use db::*;
use diagnostics::ErrorDiagnostic;
pub use req::*;
pub use req_info::*;
pub use req_state::*;
pub use rev::*;
pub use rt::*;
pub use rt_st_incr::*;
use std::{
    collections::HashMap,
    num::NonZero,
    time::{Duration, Instant},
};
use std_ext::{SmallVec1, SmallVec4};
pub use wake_after::*;

fn main() {
    let bump = bumpalo::Bump::default();
    let mut runtime = StIncrRt::new(&bump);
    runtime.set_input(
        Req::RootFile,
        NumberedRevision::default(),
        Artifact::String("Hi, world!".into()),
    );
    let result = runtime.block_on(Req::BuildExecutable, TimeoutNever);
    let _ = dbg!(&result);
}
