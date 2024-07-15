use crate::llvm_backend::address::Address;
use derive_more::IsVariant;
use llvm_sys::prelude::LLVMValueRef;

#[derive(IsVariant)]
pub enum ParamValue {
    Direct(LLVMValueRef),
    Indirect(Address),
}

impl ParamValue {
    pub fn value(&self) -> LLVMValueRef {
        match self {
            ParamValue::Direct(value) => *value,
            ParamValue::Indirect(address) => {
                assert!(address.offset.is_none());
                address.base_pointer()
            }
        }
    }
}
