use derive_more::IsVariant;
use serde::{Deserialize, Serialize};
use util_text::{ColumnSpacingAtom, LineSpacingAtom};

#[derive(Clone, Debug, Serialize, Deserialize, IsVariant)]
pub enum BareSyntaxKind {
    Root,
    Error,
    ColumnSpacing(ColumnSpacingAtom),
    LineSpacing(LineSpacingAtom),
    Punct(char),
    Null,
    True,
    False,
    Number,
    String,
    Array,
    Value,
    Identifier(Box<str>),
}
