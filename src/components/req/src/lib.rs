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
mod top_errors;
mod un_like;
mod unblock;

pub use as_syms::*;
pub use block_on::*;
pub use errors::*;
pub use is_div::*;
pub use like::*;
pub use pf::*;
pub use requests::*;
pub use rt::*;
pub use rt_st_in::*;
use serde::{Deserialize, Serialize};
use std::{path::Path, sync::Arc};
pub use succ::*;
pub use syms::*;
pub use task::*;
pub use top_errors::*;
pub use un_like::*;
pub use unblock::*;

#[macro_export]
macro_rules! log {
    () => {
        $crate::log!("\n")
    };
    ($($arg:tt)*) => {{
        if true {
            eprintln!($($arg)*);
        } else {
            let _ = format_args!($($arg)*);
        }
    }};
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct Project {
    pub root: Arc<Path>,
    pub interval_ms: Option<u64>,
    pub max_idle_time_ms: Option<u64>,
    pub cache_to_disk: Option<bool>,
}

#[define_requests::group]
mod requests {
    use super::*;

    #[define_requests::impure]
    #[define_requests::never_persist]
    #[define_requests::returns(Result<Arc<str>, TopErrors>)]
    pub struct FindProjectConfig {
        pub working_directory: Arc<Path>,
    }
    #[derive(Default)]
    pub struct FindProjectConfigState;

    #[define_requests::never_persist]
    #[define_requests::returns(Result<Project, TopErrors>)]
    pub struct GetProject {
        pub working_directory: Arc<Path>,
    }
    #[derive(Default)]
    pub struct GetProjectState;

    #[define_requests::returns(String)]
    pub struct Search();
    #[derive(Default)]
    pub struct SearchState;

    #[define_requests::never_persist]
    #[define_requests::returns(WithErrors<Syms<P>>)]
    pub struct Approach;
    #[derive(Default)]
    pub struct ApproachState;

    #[define_requests::returns(WithErrors<Vec<String>>)]
    pub struct ListSymbols;
    #[derive(Default)]
    pub struct ListSymbolsState;
}
