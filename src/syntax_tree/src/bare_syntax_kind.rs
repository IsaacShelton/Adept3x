use derive_more::IsVariant;
use serde::{Deserialize, Serialize};
use token::Punct;
use util_text::{ColumnSpacingAtom, LineSpacingAtom};

#[derive(Clone, Debug, Serialize, Deserialize, IsVariant)]
pub enum BareSyntaxKind {
    Root,
    Error { description: String },
    ColumnSpacing(ColumnSpacingAtom),
    LineSpacing(LineSpacingAtom),
    Punct(Punct),
    Null,
    True,
    False,
    Number,
    String,
    Array,
    Value,
    Identifier(Box<str>),
    Binding,
}
