use crate::data_units::ByteUnits;
use llvm_sys::{
    core::{LLVMGetTypeKind, LLVMTypeOf},
    prelude::{LLVMTypeRef, LLVMValueRef},
    LLVMTypeKind,
};

#[derive(Clone)]
pub struct RawAddress {
    pub base: LLVMValueRef,
    pub nullable: bool,
    pub alignment: ByteUnits,
    pub element_type: LLVMTypeRef,
}

impl RawAddress {
    pub fn base_pointer(&self) -> LLVMValueRef {
        self.base
    }

    pub fn pointer_type(&self) -> LLVMTypeRef {
        unsafe {
            let ty = LLVMTypeOf(self.base);
            assert_eq!(LLVMGetTypeKind(ty), LLVMTypeKind::LLVMPointerTypeKind);
            ty
        }
    }

    pub fn element_type(&self) -> LLVMTypeRef {
        self.element_type
    }
}
