mod block_on;
mod errors;
mod is_div;
mod like;
mod pf;
mod rt;
mod run;
mod succ;
mod syms;
mod task;
mod top_errors;
mod un_like;
mod unblock;

pub use block_on::*;
use by_address::ByAddress;
pub use errors::*;
pub use is_div::*;
pub use like::*;
pub use pf::*;
pub use requests::*;
pub use rt::*;
use serde::{Deserialize, Serialize};
use std::{
    marker::PhantomData,
    path::{Path, PathBuf},
    sync::Arc,
};
pub use succ::*;
pub use syms::*;
use syntax_tree::SyntaxNode;
pub use task::*;
pub use top_errors::*;
pub use un_like::*;
pub use unblock::*;

#[macro_export]
macro_rules! rt_trace {
    () => {
        $crate::log!("\n")
    };
    ($($arg:tt)*) => {{
        if false {
            log::trace!($($arg)*);
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
    use vfs::Canonical;

    #[define_requests::never_persist]
    #[define_requests::returns(Arc<str>)]
    pub struct Compile();
    #[derive(Default)]
    pub struct CompileState;

    #[define_requests::never_persist]
    #[define_requests::returns(WithErrors<Option<ByAddress<Arc<SyntaxNode>>>>)]
    pub struct ParseFile {
        pub filename: Arc<Canonical<PathBuf>>,
    }
    #[derive(Default)]
    pub struct ParseFileState;

    #[define_requests::returns(WithErrors<Arc<[String]>>)]
    pub struct ListSymbols {
        pub filename: Arc<Canonical<PathBuf>>,
    }
    #[derive(Default)]
    pub struct ListSymbolsState;

    #[define_requests::returns(PhantomData<P>)]
    pub struct UnusedRequest;
    #[derive(Default)]
    pub struct UnusedRequestState;
}
