use super::raw_address::RawAddress;
use crate::data_units::ByteUnits;
use llvm_sys::prelude::{LLVMTypeRef, LLVMValueRef};

pub struct Address {
    pub base: RawAddress,
    pub offset: Option<LLVMValueRef>,
}

impl Address {
    pub fn base_pointer(&self) -> LLVMValueRef {
        self.base.base_pointer()
    }

    pub fn pointer_type(&self) -> LLVMTypeRef {
        self.base.pointer_type()
    }

    pub fn element_type(&self) -> LLVMTypeRef {
        self.base.element_type()
    }

    pub fn alignment_at_offset(&self, offset: &ByteUnits) -> ByteUnits {
        self.base.alignment.alignment_at_offset(offset)
    }
}

impl From<RawAddress> for Address {
    fn from(base: RawAddress) -> Self {
        Self { base, offset: None }
    }
}
