use derive_more::IsVariant;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, IsVariant)]
pub enum BareSyntaxKind {
    Error,
    Whitespace,
    Punct(char),
    Null,
    True,
    False,
    Number,
    String,
    Array,
    Value,
}
