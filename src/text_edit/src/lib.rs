use derive_more::{Add, AddAssign, Sub, SubAssign, Sum};
use serde::{Deserialize, Serialize};
use std::{fmt::Display, sync::Arc};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DocumentChange {
    IncrementalUtf16(TextEditUtf16),
    Full(Arc<str>),
}
