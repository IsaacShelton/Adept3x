mod common;
mod display;

use super::{AnonymousEnum, AnonymousStruct, AnonymousUnion, FixedArray, FuncPtr, Type, TypeArg};
use crate::NamePath;
use primitives::{CInteger, FloatSize, IntegerBits, IntegerSign};
use source_files::Source;

#[derive(Clone, Debug)]
pub enum TypeKind {
    Boolean,
    Integer(IntegerBits, IntegerSign),
    CInteger(CInteger, Option<IntegerSign>),
    SizeInteger(IntegerSign),
    Floating(FloatSize),
    Ptr(Box<Type>),
    Deref(Box<Type>),
    FixedArray(Box<FixedArray>),
    Void,
    Never,
    Named(NamePath, Vec<TypeArg>),
    AnonymousStruct(AnonymousStruct),
    AnonymousUnion(AnonymousUnion),
    AnonymousEnum(AnonymousEnum),
    FuncPtr(FuncPtr),
    Polymorph(String),
}

impl TypeKind {
    pub fn at(self, source: Source) -> Type {
        Type { kind: self, source }
    }

    pub fn allow_indirect_undefined(&self) -> bool {
        if let TypeKind::Named(name, arguments) = self {
            let basename = name.basename();
            if arguments.len() == 0
                && (basename.starts_with("struct<")
                    || basename.starts_with("union<")
                    || basename.starts_with("enum<"))
            {
                return true;
            }
        }

        false
    }
}
