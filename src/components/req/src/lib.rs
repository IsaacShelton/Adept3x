mod as_syms;
mod block_on;
mod errors;
mod is_div;
mod like;
mod pf;
mod rt;
mod rt_st_in;
mod run;
mod succ;
mod syms;
mod task;
mod un_like;
mod unblock;

pub use as_syms::*;
pub use block_on::*;
use by_address::ByAddress;
pub use errors::*;
pub use is_div::*;
pub use like::*;
pub use pf::*;
pub use requests::*;
pub use rt::*;
pub use rt_st_in::*;
use std::{path::Path, sync::Arc};
pub use succ::*;
pub use syms::*;
pub use task::*;
pub use un_like::*;
pub use unblock::*;
use vfs::Vfs;

#[derive(Debug, Hash, PartialEq, Eq)]
pub struct Project {
    root: Arc<Path>,
}

#[define_requests::group]
mod requests {
    use super::*;

    #[define_requests::impure]
    #[define_requests::returns(Result<Arc<str>, Arc<Errs>>)]
    pub struct FindProjectConfig {
        pub working_directory: Arc<Path>,
    }
    #[derive(Default)]
    pub struct FindProjectConfigState;

    #[define_requests::returns(Result<Arc<Project>, Arc<Errs>>)]
    pub struct GetProject {
        pub working_directory: Arc<Path>,
    }
    #[derive(Default)]
    pub struct GetProjectState;

    #[define_requests::returns(String)]
    pub struct GetRootSourceFile;
    #[derive(Default)]
    pub struct GetRootSourceFileState;

    #[define_requests::returns(String)]
    pub struct Search<'e>(&'e str);
    #[derive(Default)]
    pub struct SearchState;

    #[define_requests::returns(Syms<P>)]
    pub struct Approach;
    #[derive(Default)]
    pub struct ApproachState;

    #[define_requests::returns(Vec<String>)]
    pub struct ListSymbols {
        pub vfs: ByAddress<Arc<Vfs>>,
    }
    #[derive(Default)]
    pub struct ListSymbolsState;
}
