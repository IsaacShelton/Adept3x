use super::{
    AnonymousEnum, CInteger, FixedArray, FloatSize, FunctionPointer, IntegerBits, IntegerSign,
    Privacy, StructureRef,
};
use crate::source_files::Source;
use derive_more::{IsVariant, Unwrap};
use num::BigInt;
use slotmap::new_key_type;

new_key_type! {
    pub struct TypeRef;
    pub struct EnumRef;
}

#[derive(Clone, Debug)]
pub struct Type {
    pub value: TypeRef,
    pub source: Source,
}

#[derive(Clone, Debug)]
pub struct TypeDecl {
    pub kind: TypeKind,
    pub source: Source,
    pub privacy: Privacy,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct HumanName(pub String);

#[derive(Clone, Debug, PartialEq, IsVariant, Unwrap)]
pub enum TypeKind {
    Unresolved,
    Boolean,
    Integer(IntegerBits, IntegerSign),
    CInteger(CInteger, Option<IntegerSign>),
    IntegerLiteral(BigInt),
    FloatLiteral(f64),
    Floating(FloatSize),
    Pointer(Box<TypeRef>),
    Void,
    AnonymousStruct(),
    AnonymousUnion(),
    AnonymousEnum(AnonymousEnum),
    FixedArray(Box<FixedArray>),
    FunctionPointer(FunctionPointer),
    Enum(HumanName, TypeRef),
    Structure(HumanName, StructureRef),
    TypeAlias(HumanName, TypeRef),
}
