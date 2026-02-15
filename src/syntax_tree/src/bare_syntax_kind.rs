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
    BuiltinValue(BuiltinValue),
    BuiltinType(BuiltinType),
    Number,
    String,
    Array,
    Term,
    Identifier(Box<str>),
    Fn,
    ParamList,
    Param,
    Name,
    Binding,
}

#[derive(Clone, Debug, Serialize, Deserialize, IsVariant)]
pub enum BuiltinType {
    Bool,
    Void,
    Type,
}

#[derive(Clone, Debug, Serialize, Deserialize, IsVariant)]
pub enum BuiltinValue {
    True,
    False,
    Void,
}
