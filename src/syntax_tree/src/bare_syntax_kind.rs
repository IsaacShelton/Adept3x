use derive_more::IsVariant;
use serde::{Deserialize, Serialize};
use token::{Directive, IsTerminated, Punct};
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
    Directive(Directive),
    Fn,
    ParamList,
    Param,
    Block,
    Name,
    Binding,
    Variable(Box<str>),
    If,
    IfArgList,
    SinglelineComment(Box<str>),
    MultilineComment(Box<str>, IsTerminated),
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
