use derive_more::{IsVariant, PartialEq};
use num_bigint::BigInt;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use token::{Directive, Punct};
use util_text::{ColumnSpacingAtom, LineSpacingAtom};

#[derive(Clone, Debug, Serialize, Deserialize, IsVariant, PartialEq, Eq)]
pub enum BareSyntaxKind {
    Root,
    Error { description: String },
    ColumnSpacing(ColumnSpacingAtom),
    LineSpacing(LineSpacingAtom),
    Punct(Punct),
    Term,
    Identifier(Arc<str>),
    Directive(Directive),
    ParamList,
    Param,
    ParamHead,
    Name,
    ImplicitName,
    Binding,
    ArgList(Reparsable),
    FieldDef,
    FieldDefList,
    TypeAnnotation,
    SinglelineComment(Box<str>),
    MultilineComment(Box<str>),
    BuiltinType(BuiltinType),
    TrueValue,
    FalseValue,
    VoidValue,
    Integer(Arc<BigInt>),
    FnValue,
    IfValue,
    RecordValue,
    Block,
    Variable(Arc<str>),
    Eval,
    ParenthesizedTerm,
    Call,
    Let,
    Nth,
    Match,
    MatchBlock,
    MatchArm,
    Pattern,
    BoolElim,
    NatElim,
    NatSucc,
}

#[derive(Clone, Debug, Serialize, Deserialize, IsVariant, PartialEq, Eq)]
pub enum Reparsable {
    Reparse,
    Ignore,
}

#[derive(Clone, Debug, Serialize, Deserialize, IsVariant, PartialEq, Eq)]
pub enum BuiltinType {
    Bool,
    Void,
    Type,
    Fn,
    Record,
    Nat,
}
