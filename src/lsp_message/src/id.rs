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

impl LspRequestId {
    pub fn succ(&self) -> Self {
        match self {
            LspRequestId::Int(id) => {
                if let Some(new_id) = id.checked_add(1) {
                    Self::Int(new_id)
                } else {
                    Self::String(format!("{}", i32::MAX as u32 + 1).into())
                }
            }
            LspRequestId::String(id) => match usize::from_str_radix(id, 10) {
                Ok(id) => {
                    let new_id = id.checked_add(1).expect("exhausted possible request ids");
                    Self::String(format!("{}", new_id).into())
                }
                Err(_) => unreachable!(),
            },
        }
    }
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
