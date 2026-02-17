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
    Term,
    Identifier(Box<str>),
    Directive(Directive),
    ParamList,
    Param,
    Name,
    Binding,
    IfArgList,
    FieldDef,
    FieldDefList,
    TypeAnnotation,
    SinglelineComment(Box<str>),
    MultilineComment(Box<str>, IsTerminated),
    BuiltinType(BuiltinType),
    TrueValue,
    FalseValue,
    VoidValue,
    FnValue,
    IfValue,
    Block,
    Variable(Box<str>),
}

#[derive(Clone, Debug, Serialize, Deserialize, IsVariant)]
pub enum BuiltinType {
    Bool,
    Void,
    Type,
    Fn,
    Record,
}
