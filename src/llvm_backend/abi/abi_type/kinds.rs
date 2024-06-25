use super::offset_align::{ByteCount, OffsetAlign};
use core::fmt::Debug;
use llvm_sys::{core::LLVMPrintTypeToString, prelude::LLVMTypeRef};
use std::ffi::CString;

#[derive(Clone)]
pub struct Direct {
    pub offset_align: OffsetAlign,
    pub coerce_to_type: Option<LLVMTypeRef>,
    pub padding: Option<LLVMTypeRef>,
    pub in_register: bool,
    pub can_be_flattened: bool,
}

struct ShowLLVMType(LLVMTypeRef);

impl Debug for ShowLLVMType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let representation = unsafe { CString::from_raw(LLVMPrintTypeToString(self.0)) };
        write!(f, "LLVMTypeRef::{}", representation.to_str().unwrap())
    }
}

impl Debug for Direct {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Direct")
            .field("offset_align", &self.offset_align)
            .field("coerce_to_type", &self.coerce_to_type.map(ShowLLVMType))
            .field("padding", &self.padding.map(ShowLLVMType))
            .field("in_register", &self.in_register)
            .field("can_be_flattened", &self.can_be_flattened)
            .finish()
    }
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
