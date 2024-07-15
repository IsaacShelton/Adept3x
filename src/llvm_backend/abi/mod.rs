use crate::ir;

pub mod abi_function;
pub mod abi_type;
pub mod arch;
mod cxx;
mod empty;

pub fn has_scalar_evaluation_kind(ty: &ir::Type) -> bool {
    match ty {
        ir::Type::Pointer(_)
        | ir::Type::Boolean
        | ir::Type::S8
        | ir::Type::S16
        | ir::Type::S32
        | ir::Type::S64
        | ir::Type::U8
        | ir::Type::U16
        | ir::Type::U32
        | ir::Type::U64
        | ir::Type::F32
        | ir::Type::F64
        | ir::Type::Vector(_) => true,
        ir::Type::Atomic(inner) => has_scalar_evaluation_kind(inner),
        ir::Type::Complex(_)
        | ir::Type::Void
        | ir::Type::Union(_)
        | ir::Type::Structure(_)
        | ir::Type::AnonymousComposite(_)
        | ir::Type::FunctionPointer
        | ir::Type::FixedArray(_)
        | ir::Type::IncompleteArray(_) => false,
    }
}
