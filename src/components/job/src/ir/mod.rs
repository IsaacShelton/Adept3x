mod field;
mod instr;
mod value;

pub use crate::ir::field::Field;
use arena::{Idx, LockFreeArena, new_id_with_niche};
use attributes::SymbolOwnership;
use derivative::Derivative;
use derive_more::IsVariant;
pub use instr::*;
use source_files::Source;
use std::sync::OnceLock;
pub use value::*;

new_id_with_niche!(FuncId, u32);
new_id_with_niche!(StructId, u32);
new_id_with_niche!(GlobalId, u32);

pub type FuncRef<'env> = Idx<FuncId, Func<'env>>;
pub type StructRef<'env> = Idx<StructId, Struct<'env>>;
pub type GlobalRef<'env> = Idx<GlobalId, Global<'env>>;

#[derive(Clone, Debug)]
pub struct Struct<'env> {
    pub name: &'env str,
    pub fields: &'env [&'env Field<'env>],
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
    pub return_type: Type<'env>,
    pub is_cstyle_variadic: bool,
    pub ownership: SymbolOwnership,
    pub abide_abi: bool,
    pub basicblocks: OnceLock<&'env [BasicBlock<'env>]>,
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
    AnonymousComposite(&'env TypeComposite<'env>),
    Union(()),
    FixedArray(FixedArray<'env>),
    Vector(Vector<'env>),
    Complex(Complex<'env>),
    Atomic(&'env Type<'env>),
    IncompleteArray(&'env Type<'env>),
}

impl<'env> Type<'env> {
    pub fn is_fixed_vector(&self) -> bool {
        // NOTE: We don't support fixed vector types yet
        false
    }

    pub fn is_product_type(&self) -> bool {
        self.is_struct() || self.is_anonymous_composite()
    }

    pub fn has_flexible_array_member(&self) -> bool {
        // NOTE: We don't support flexible array members yet
        false
    }

    pub fn is_signed(&self) -> Option<bool> {
        match self {
            Type::S8 | Type::S16 | Type::S32 | Type::S64 => Some(true),
            Type::Bool | Type::U8 | Type::U16 | Type::U32 | Type::U64 => Some(false),
            _ => None,
        }
    }

    pub fn is_integer_like(&self) -> bool {
        matches!(
            self,
            Type::Bool
                | Type::S8
                | Type::S16
                | Type::S32
                | Type::S64
                | Type::U8
                | Type::U16
                | Type::U32
                | Type::U64
        )
    }

    pub fn is_builtin_data(&self) -> bool {
        match self {
            Type::Bool
            | Type::S8
            | Type::S16
            | Type::S32
            | Type::S64
            | Type::U8
            | Type::U16
            | Type::U32
            | Type::U64
            | Type::F32
            | Type::F64 => true,
            Type::Ptr(_)
            | Type::Void
            | Type::Struct(_)
            | Type::AnonymousComposite(_)
            | Type::Union(_)
            | Type::FuncPtr
            | Type::FixedArray(_)
            | Type::Vector(_)
            | Type::Complex(_)
            | Type::Atomic(_)
            | Type::IncompleteArray(_) => false,
        }
    }

    pub fn struct_fields(&self, ir_module: &'env Ir<'env>) -> Option<&'env [&'env Field<'env>]> {
        match self {
            Type::Struct(struct_ref) => Some(&ir_module.structs[*struct_ref].fields[..]),
            /*Type::AnonymousComposite(composite) => Some(&composite.fields[..]),*/
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Derivative)]
#[derivative(PartialEq, Eq, Hash)]
pub struct TypeComposite<'env> {
    pub fields: &'env [&'env Field<'env>],
    pub is_packed: bool,

    #[derivative(Hash = "ignore")]
    #[derivative(PartialEq = "ignore")]
    pub source: Source,
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
