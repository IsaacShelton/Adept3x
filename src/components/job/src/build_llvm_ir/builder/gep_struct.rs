use super::Builder;
use crate::build_llvm_ir::{
    address::Address, llvm_type_ref_ext::LLVMTypeRefExt, raw_address::RawAddress,
    target_data::TargetData,
};
use data_units::ByteUnits;
use llvm_sys::{
    LLVMTypeKind,
    core::{LLVMBuildStructGEP2, LLVMGetTypeKind},
    prelude::LLVMTypeRef,
    target::LLVMOffsetOfElement,
};

impl<'env> Builder<'env> {
    pub fn gep_struct(
        &mut self,
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
                c"".as_ptr(),
            )
        };

        let alignment = address.alignment_at_offset(&offset);

        let field_type = precomputed_field_types
            .map(|fields| fields[index as usize])
            .unwrap_or_else(|| struct_type.field_types()[index as usize]);

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
