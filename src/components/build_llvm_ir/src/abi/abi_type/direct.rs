use data_units::ByteUnits;
use llvm_sys::prelude::LLVMTypeRef;

#[derive(Clone, Debug)]
pub struct DirectOptions {
    pub coerce_to_type: Option<LLVMTypeRef>,
    pub offset: ByteUnits,
    pub padding: Option<LLVMTypeRef>,
    pub can_be_flattened: bool,
    pub alignment: ByteUnits,
    pub in_register: bool,
}

impl Default for DirectOptions {
    fn default() -> Self {
        Self {
            coerce_to_type: None,
            offset: ByteUnits::of(0),
            can_be_flattened: true,
            padding: None,
            alignment: ByteUnits::of(0),
            in_register: false,
        }
    }
}
