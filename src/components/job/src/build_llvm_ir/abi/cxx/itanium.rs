use super::super::abi_type::ABIType;
use crate::{ir, target_layout::TypeLayoutCache};
use derive_more::IsVariant;
use target::Target;

#[derive(Clone, Debug)]
pub struct Itanium<'a, 'env> {
    pub target: &'a Target,
    pub type_layout_cache: &'a TypeLayoutCache<'env>,
}

pub fn can_pass_in_registers_composite(ty: &ir::Type) -> Option<bool> {
    ty.is_product_type().then_some(true)
}

impl<'a, 'env> Itanium<'a, 'env> {
    pub fn new(type_layout_cache: &'a TypeLayoutCache<'env>, target: &'a Target) -> Self {
        Self {
            type_layout_cache,
            target,
        }
    }

    pub fn classify_return_type(&self, return_type: &'env ir::Type<'env>) -> Option<ABIType> {
        if let Some(false) = can_pass_in_registers_composite(return_type) {
            // NOTE: For returning types that aren't C++ copyable, we
            // should return by address.

            // This doesn't apply to any of our types yet.

            let align = self.type_layout_cache.get(return_type).alignment;

            Some(ABIType::new_indirect(
                align,
                Some(false),
                None,
                None,
                Default::default(),
            ))
        } else {
            None
        }
    }

    pub fn get_record_arg_abi(&self, ty: &ir::Type) -> RecordArgABI {
        if !can_pass_in_registers_composite(ty).unwrap_or(false) {
            RecordArgABI::Indirect
        } else {
            RecordArgABI::Default
        }
    }
}

#[derive(Copy, Clone, Debug, IsVariant)]
pub enum RecordArgABI {
    Default,
    DirectInMemory,
    Indirect,
}
