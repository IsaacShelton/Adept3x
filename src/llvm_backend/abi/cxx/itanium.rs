use super::super::abi_type::ABIType;
use crate::{
    ir,
    target_info::{type_info::TypeInfoManager, TargetInfo},
};

#[derive(Clone, Debug)]
pub struct Itanium<'a> {
    pub target_info: &'a TargetInfo,
    pub type_info_manager: &'a TypeInfoManager<'a>,
}

pub fn can_pass_in_registers_composite(ty: &ir::Type) -> Option<bool> {
    match ty {
        ir::Type::Structure(..) | ir::Type::AnonymousComposite(..) => Some(true),
        _ => None,
    }
}

impl<'a> Itanium<'a> {
    pub fn new(type_info_manager: &'a TypeInfoManager, target_info: &'a TargetInfo) -> Self {
        Self {
            type_info_manager,
            target_info,
        }
    }
    pub fn classify_return_type(&self, return_type: &ir::Type) -> Option<ABIType> {
        if let Some(false) = can_pass_in_registers_composite(return_type) {
            // NOTE: For returning types that aren't C++ copiable, we
            // should return by address.

            // This doesn't apply to any of our types yet.

            let align = self
                .type_info_manager
                .get_type_info(return_type, self.target_info)
                .alignment;

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
}
