use super::offset_align::{ByteCount, OffsetAlign};
use llvm_sys::prelude::LLVMTypeRef;

#[derive(Clone, Debug)]
pub struct Direct {
    pub offset_align: OffsetAlign,
    pub coerce_to_type: Option<LLVMTypeRef>,
    pub padding: Option<LLVMTypeRef>,
    pub in_register: bool,
    pub can_be_flattened: bool,
}

#[derive(Clone, Debug)]
pub struct Extend {
    pub offset_align: OffsetAlign,
    pub coerce_to_type: Option<LLVMTypeRef>,
    pub padding: Option<LLVMTypeRef>,
    pub in_register: bool,
    pub signext: bool,
}

#[derive(Clone, Debug)]
pub struct Indirect {
    pub padding: Option<LLVMTypeRef>,
    pub align: ByteCount,
    pub byval: bool,
    pub realign: bool,
    pub sret_after_this: bool,
    pub in_register: bool,
}

#[derive(Clone, Debug)]
pub struct IndirectAliased {
    pub padding: Option<LLVMTypeRef>,
    pub align: ByteCount,
    pub realign: bool,
    pub address_space: u32,
}

#[derive(Clone, Debug)]
pub struct Expand {
    pub padding: Option<LLVMTypeRef>,
}

#[derive(Clone, Debug)]
pub struct InAlloca {
    pub alloca_field_index: u32,
    pub sret: bool,
    pub indirect: bool,
}

#[derive(Clone, Debug)]
pub struct CoerceAndExpand {
    pub coerce_to_type: LLVMTypeRef,
    pub unpadded_coerce_and_expand_type: LLVMTypeRef,
}
