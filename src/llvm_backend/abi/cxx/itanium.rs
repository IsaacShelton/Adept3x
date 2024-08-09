use super::super::abi_type::ABIType;
use crate::{
    ir,
    target_info::{type_layout::TypeLayoutCache, TargetInfo},
};
use derive_more::IsVariant;

#[derive(Clone, Debug)]
pub struct Itanium<'a> {
    pub target_info: &'a TargetInfo,
    pub type_layout_cache: &'a TypeLayoutCache<'a>,
}

pub fn can_pass_in_registers_composite(ty: &ir::Type) -> Option<bool> {
    ty.is_product_type().then_some(true)
}

impl<'a> Itanium<'a> {
    pub fn new(type_layout_cache: &'a TypeLayoutCache, target_info: &'a TargetInfo) -> Self {
        Self {
            type_layout_cache,
            target_info,
        }
    }

    pub fn classify_return_type(&self, return_type: &ir::Type) -> Option<ABIType> {
        if let Some(false) = can_pass_in_registers_composite(return_type) {
            // NOTE: For returning types that aren't C++ copiable, we
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
