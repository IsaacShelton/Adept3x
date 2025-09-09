pub mod abi_function;
pub mod abi_type;
pub mod arch;
mod cxx;
pub mod empty;
mod homo_aggregate;

use crate::ir;
use primitives::IntegerBits;

pub fn has_scalar_evaluation_kind(ty: &ir::Type) -> bool {
    match ty {
        ir::Type::Ptr(_)
        | ir::Type::Bool
        | ir::Type::I(..)
        | ir::Type::F(..)
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
        ir::Type::Bool | ir::Type::I(IntegerBits::Bits8 | IntegerBits::Bits16, _) => true,
        ir::Type::I(IntegerBits::Bits32 | IntegerBits::Bits64, _)
        | ir::Type::F(..)
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
