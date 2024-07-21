use super::Builder;
use crate::{
    data_units::ByteUnits,
    llvm_backend::{address::Address, raw_address::RawAddress, target_data::TargetData},
};
use cstr::cstr;
use llvm_sys::core::{LLVMBuildInBoundsGEP2, LLVMConstInt, LLVMInt64Type};

impl Builder {
    pub fn gep_in_bounds(
        &self,
        target_data: &TargetData,
        address: &Address,
        index: u64,
    ) -> Address {
        let element_type = address.element_type();
        let element_size = ByteUnits::of(
            target_data
                .abi_size_of_type(element_type)
                .try_into()
                .unwrap(),
        );

        let mut indices = [unsafe { LLVMConstInt(LLVMInt64Type(), index, false as _) }];

        let base = unsafe {
            LLVMBuildInBoundsGEP2(
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
            alignment: address.alignment_at_offset(&(ByteUnits::of(index) * element_size)),
            element_type,
        }
        .into()
    }
}
