use super::Builder;
use crate::{
    data_units::ByteUnits,
    llvm_backend::{
        address::Address, llvm_value_ref_ext::LLVMValueRefExt, raw_address::RawAddress,
        target_data::TargetData,
    },
};
use cstr::cstr;
use llvm_sys::{core::LLVMBuildGEP2, prelude::LLVMValueRef};

impl Builder {
    pub fn gep(
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
            LLVMBuildGEP2(
                self.get(),
                element_type,
                address.base_pointer(),
                indices.as_mut_ptr(),
                indices.len() as _,
                cstr!("").as_ptr(),
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
