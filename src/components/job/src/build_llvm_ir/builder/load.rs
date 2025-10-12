use super::Builder;
use crate::build_llvm_ir::address::Address;
use data_units::ByteUnits;
use derive_more::IsVariant;
use llvm_sys::{
    core::{LLVMBuildLoad2, LLVMSetAlignment, LLVMSetVolatile},
    prelude::{LLVMTypeRef, LLVMValueRef},
};
use std::ffi::CStr;

#[derive(Copy, Clone, IsVariant)]
pub enum Volatility {
    Normal,
    Volitile,
}

impl<'env> Builder<'env> {
    pub fn load(&mut self, address: &Address, volatility: Volatility) -> LLVMValueRef {
        self.load_aligned(
            address.element_type(),
            address.base_pointer(),
            address.base.alignment,
            volatility,
            c"",
        )
    }

    pub fn load_aligned(
        &mut self,
        ty: LLVMTypeRef,
        pointer: LLVMValueRef,
        alignment: ByteUnits,
        volatility: Volatility,
        name: &CStr,
    ) -> LLVMValueRef {
        unsafe {
            let load = LLVMBuildLoad2(self.get(), ty, pointer, name.as_ptr());
            LLVMSetAlignment(load, alignment.bytes().try_into().unwrap());
            LLVMSetVolatile(load, i32::from(volatility.is_volitile()));
            load
        }
    }
}
