use crate::{
    ir,
    llvm_backend::abi::{abi_function::ABIFunction, abi_type::ABIType},
};
use derive_more::IsVariant;
use llvm_sys::{prelude::LLVMTypeRef, LLVMCallConv};

#[derive(Clone, Debug)]
pub struct AARCH64 {
    pub variant: Variant,
}

#[derive(Clone, Debug, IsVariant)]
pub enum Variant {
    DarwinPCS,
    Win64,
    AAPCS,
    AAPCSSoft,
}

#[allow(unused)]
impl AARCH64 {
    pub fn function(
        &self,
        original_parameter_types: &[LLVMTypeRef],
        original_return_type: Option<LLVMTypeRef>,
    ) -> ABIFunction {
        Self::compute_info(original_parameter_types, original_return_type)
    }

    fn compute_info(
        _original_parameter_types: &[LLVMTypeRef],
        _original_return_type: Option<LLVMTypeRef>,
    ) -> ABIFunction {
        todo!("compute_info")
    }

    fn is_soft(&self) -> bool {
        self.variant.is_aapcs_soft()
    }

    fn classify_return_type(&self, return_type: &ir::Type, is_variadic: bool) -> ABIType {
        todo!("classify_return_type")
    }

    fn classify_argument_type(
        &self,
        return_type: &ir::Type,
        ir_variadic: bool,
        calling_convention: LLVMCallConv,
    ) -> ABIType {
        todo!("classify_argument_type")
    }

    fn coerce_illegal_vector(&self, ty: &ir::Type) -> ABIType {
        todo!("coerce_illegal_vector")
    }

    fn is_homo_aggregate_base_type(&self, ty: &ir::Type) -> bool {
        todo!("is_homo_aggregate_base_type")
    }

    fn is_homo_aggregate_small_enough(&self, ty: &ir::Type, members: u64) -> bool {
        todo!("is_homo_aggregate_small_enough")
    }

    fn is_zero_length_bitfield_allowed_in_homo_aggregrate(&self) -> bool {
        todo!("is_zero_length_bitfield_allowed_in_homo_aggregrate")
    }

    fn is_illegal_vector_type(&self, ty: &ir::Type) -> bool {
        todo!("is_illegal_vector_type")
    }
}
