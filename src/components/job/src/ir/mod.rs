mod field;
mod instr;
mod value;

use crate::ir::{field::Field, instr::Instr};
use arena::{Idx, LockFreeArena, new_id_with_niche};
use attributes::SymbolOwnership;
use derive_more::IsVariant;
use source_files::Source;

new_id_with_niche!(FuncId, u32);
new_id_with_niche!(StructId, u32);
new_id_with_niche!(GlobalId, u32);

pub type FuncRef<'env> = Idx<FuncId, Func<'env>>;
pub type StructRef<'env> = Idx<StructId, Struct<'env>>;
pub type GlobalRef<'env> = Idx<GlobalId, Global<'env>>;

#[derive(Clone, Debug)]
pub struct Struct<'env> {
    pub name: &'env str,
    pub fields: &'env [Field<'env>],
    pub is_packed: bool,
    pub source: Source,
}

#[derive(Clone, Debug)]
pub struct Global<'env> {
    pub mangled_name: &'env str,
    pub ir_type: Type<'env>,
    pub is_thread_local: bool,
    pub ownership: SymbolOwnership,
}

#[derive(Debug, Default)]
pub struct Ir<'env> {
    pub funcs: LockFreeArena<FuncId, Func<'env>>,
    pub structs: LockFreeArena<StructId, Struct<'env>>,
    pub globals: LockFreeArena<GlobalId, Global<'env>>,
}

#[derive(Clone, Debug)]
pub struct Func<'env> {
    pub mangled_name: &'env str,
    pub params: &'env [Type<'env>],
    pub return_type: &'env Type<'env>,
    //pub basicblocks: BasicBlocks,
    pub is_cstyle_variadic: bool,
    pub ownership: SymbolOwnership,
    pub abide_abi: bool,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, IsVariant, Hash)]
pub enum Type<'env> {
    Ptr(&'env Type<'env>),
    Bool,
    S8,
    S16,
    S32,
    S64,
    U8,
    U16,
    U32,
    U64,
    F32,
    F64,
    Void,
    FuncPtr,
    Struct(StructRef<'env>),
    FixedArray(FixedArray<'env>),
    Vector(Vector<'env>),
    Complex(Complex<'env>),
    Atomic(&'env Type<'env>),
    IncompleteArray(&'env Type<'env>),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct FixedArray<'env> {
    pub length: u64,
    pub inner: &'env Type<'env>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Vector<'env> {
    pub element_type: &'env Type<'env>,
    pub num_elements: u64,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Complex<'env> {
    pub element_type: &'env Type<'env>,
}

#[derive(Clone, Debug, Default)]
pub struct BasicBlock<'env> {
    pub instructions: &'env [Instr<'env>],
}
