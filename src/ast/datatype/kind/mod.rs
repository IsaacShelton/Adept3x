mod common;
mod display;

use super::{
    AnonymousEnum, AnonymousStruct, AnoymousUnion, CInteger, FixedArray, FloatSize,
    FunctionPointer, IntegerBits, IntegerSign, Type,
};
use crate::{name::Name, source_files::Source};

#[derive(Clone, Debug)]
pub enum TypeKind {
    Boolean,
    Integer(IntegerBits, IntegerSign),
    CInteger(CInteger, Option<IntegerSign>),
    Floating(FloatSize),
    Pointer(Box<Type>),
    FixedArray(Box<FixedArray>),
    Void,
    Named(Name),
    AnonymousStruct(AnonymousStruct),
    AnonymousUnion(AnoymousUnion),
    AnonymousEnum(AnonymousEnum),
    FunctionPointer(FunctionPointer),
    Polymorph(String),
}

impl TypeKind {
    pub fn at(self, source: Source) -> Type {
        Type { kind: self, source }
    }

    pub fn allow_indirect_undefined(&self) -> bool {
        if let TypeKind::Named(name) = self {
            let basename = &name.basename;
            if basename.starts_with("struct<")
                || basename.starts_with("union<")
                || basename.starts_with("enum<")
            {
                return true;
            }
        }

        false
    }
}
