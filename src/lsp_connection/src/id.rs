use derive_more::From;
use serde::{Deserialize, Serialize};
use std::{
    fmt::{Display, Formatter},
    sync::Arc,
};

#[derive(Clone, Debug, From, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(untagged)]
pub enum LspRequestId {
    Int(i32),
    String(Arc<str>),
}

impl From<String> for LspRequestId {
    fn from(id: String) -> LspRequestId {
        Self::String(Arc::from(id))
    }
}

impl Display for LspRequestId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Int(it) => write!(f, "{}", it),
            Self::String(it) => write!(f, "{:?}", it),
        }
    }
}
