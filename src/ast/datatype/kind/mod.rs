mod common;
mod display;

use super::{
    AnonymousEnum, AnonymousStruct, AnoymousUnion, CInteger, FixedArray, FloatSize,
    FunctionPointer, IntegerBits, IntegerSign, Type,
};
use crate::source_files::Source;

#[derive(Clone, Debug)]
pub enum TypeKind {
    Boolean,
    Integer(IntegerBits, IntegerSign),
    CInteger(CInteger, Option<IntegerSign>),
    Floating(FloatSize),
    Pointer(Box<Type>),
    FixedArray(Box<FixedArray>),
    PlainOldData(Box<Type>),
    Void,
    Named(String),
    AnonymousStruct(AnonymousStruct),
    AnonymousUnion(AnoymousUnion),
    AnonymousEnum(AnonymousEnum),
    FunctionPointer(FunctionPointer),
}

impl TypeKind {
    pub fn at(self, source: Source) -> Type {
        Type { kind: self, source }
    }

    pub fn allow_indirect_undefined(&self) -> bool {
        if let TypeKind::Named(name) = self {
            if name.starts_with("struct<")
                || name.starts_with("union<")
                || name.starts_with("enum<")
            {
                return true;
            }
        }

        false
    }
}
