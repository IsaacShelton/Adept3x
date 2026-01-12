mod line_index;
mod text_edit_utf16;
mod text_length_utf16;
mod text_point_utf16;
mod text_range_utf16;

pub use line_index::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
pub use text_edit_utf16::*;
pub use text_length_utf16::*;
pub use text_point_utf16::*;
pub use text_range_utf16::*;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DocumentChange {
    IncrementalUtf16(TextEditUtf16),
    Full(Arc<str>),
}
