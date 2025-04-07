pub mod abi_function;
pub mod abi_type;
pub mod arch;
mod cxx;
pub mod empty;
mod homo_aggregate;

pub fn has_scalar_evaluation_kind(ty: &ir::Type) -> bool {
    match ty {
        ir::Type::Ptr(_)
        | ir::Type::Bool
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
        | ir::Type::Struct(_)
        | ir::Type::AnonymousComposite(_)
        | ir::Type::FuncPtr
        | ir::Type::FixedArray(_)
        | ir::Type::IncompleteArray(_) => false,
    }
}

pub fn is_aggregate_type_for_abi(ty: &ir::Type) -> bool {
    !has_scalar_evaluation_kind(ty)
}

pub fn is_promotable_integer_type_for_abi(ty: &ir::Type) -> bool {
    // NOTE: Arbitrarily sized integers and `char32` should be, but we don't support those yet

    match ty {
        ir::Type::Bool | ir::Type::S8 | ir::Type::S16 | ir::Type::U8 | ir::Type::U16 => true,
        ir::Type::S32
        | ir::Type::S64
        | ir::Type::U32
        | ir::Type::U64
        | ir::Type::F32
        | ir::Type::F64
        | ir::Type::Ptr(_)
        | ir::Type::Void
        | ir::Type::Union(_)
        | ir::Type::Struct(_)
        | ir::Type::AnonymousComposite(_)
        | ir::Type::FuncPtr
        | ir::Type::FixedArray(_)
        | ir::Type::Vector(_)
        | ir::Type::Complex(_)
        | ir::Type::Atomic(_)
        | ir::Type::IncompleteArray(_) => false,
    }
}
