use llvm_sys::prelude::LLVMTypeRef;

#[derive(Clone, Debug)]
pub struct DirectOptions {
    pub coerce_to_type: Option<LLVMTypeRef>,
    pub offset: u32,
    pub padding: Option<LLVMTypeRef>,
    pub can_be_flattened: bool,
    pub align_bytes: u32,
}

impl Default for DirectOptions {
    fn default() -> Self {
        Self {
            coerce_to_type: None,
            offset: 0,
            can_be_flattened: true,
            padding: None,
            align_bytes: 0,
        }
    }
}
