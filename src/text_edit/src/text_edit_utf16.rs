use crate::TextPointRangeUtf16;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextEditUtf16 {
    pub range: TextPointRangeUtf16,
    pub replace_with: Arc<str>,
}
