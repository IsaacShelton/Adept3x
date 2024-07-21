use super::Builder;
use crate::{
    data_units::ByteUnits,
    llvm_backend::{
        abi::abi_type::get_struct_field_types, address::Address, raw_address::RawAddress,
        target_data::TargetData,
    },
};
use cstr::cstr;
use llvm_sys::{
    core::{LLVMBuildStructGEP2, LLVMGetTypeKind},
    prelude::LLVMTypeRef,
    target::LLVMOffsetOfElement,
    LLVMTypeKind,
};

impl Builder {
    pub fn gep_struct(
        &self,
        target_data: &TargetData,
        address: &Address,
        index: usize,
        precomputed_field_types: Option<&[LLVMTypeRef]>,
    ) -> Address {
        let struct_type = address.element_type();
        let index = u32::try_from(index).unwrap();

        assert_eq!(
            unsafe { LLVMGetTypeKind(struct_type) },
            LLVMTypeKind::LLVMStructTypeKind
        );

        let offset =
            ByteUnits::of(unsafe { LLVMOffsetOfElement(target_data.get(), struct_type, index) });

        let base = unsafe {
            LLVMBuildStructGEP2(
                self.get(),
                address.element_type(),
                address.base_pointer(),
                index,
                cstr!("").as_ptr(),
            )
        };

        let alignment = address.alignment_at_offset(&offset);

        let field_type = precomputed_field_types
            .map(|fields| fields[index as usize])
            .unwrap_or_else(|| get_struct_field_types(struct_type)[index as usize]);

        Address {
            base: RawAddress {
                base,
                nullable: false,
                alignment,
                element_type: field_type,
            },
            offset: None,
        }
    }
}
