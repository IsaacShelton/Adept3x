use super::Builder;
use crate::build_llvm_ir::{
    address::Address, llvm_value_ref_ext::LLVMValueRefExt, raw_address::RawAddress,
    target_data::TargetData,
};
use data_units::ByteUnits;
use llvm_sys::{core::LLVMBuildInBoundsGEP2, prelude::LLVMValueRef};

impl<'env> Builder<'env> {
    pub fn gep_in_bounds(
        &self,
        target_data: &TargetData,
        address: &Address,
        field_index: u64,
        array_index: u64,
    ) -> Address {
        let element_type = address.element_type();
        let element_size = target_data.abi_size_of_type(element_type);

        let mut indices = [
            LLVMValueRef::new_u64(field_index),
            LLVMValueRef::new_u64(array_index),
        ];

        let base = unsafe {
            LLVMBuildInBoundsGEP2(
                self.get(),
                element_type,
                address.base_pointer(),
                indices.as_mut_ptr(),
                indices.len() as _,
                c"".as_ptr(),
            )
        };

        RawAddress {
            base,
            nullable: false,
            alignment: address.alignment_at_offset(&(ByteUnits::of(field_index) * element_size)),
            element_type,
        }
        .into()
    }
}
