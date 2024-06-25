use super::{
    offset_align::{ByteCount, OffsetAlign},
    show_llvm_type::ShowLLVMType,
};
use core::fmt::Debug;
use llvm_sys::prelude::LLVMTypeRef;

#[derive(Clone)]
pub struct Direct {
    pub offset_align: OffsetAlign,
    pub coerce_to_type: Option<LLVMTypeRef>,
    pub padding: Option<LLVMTypeRef>,
    pub in_register: bool,
    pub can_be_flattened: bool,
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

#[derive(Clone)]
pub struct Extend {
    pub offset_align: OffsetAlign,
    pub coerce_to_type: Option<LLVMTypeRef>,
    pub padding: Option<LLVMTypeRef>,
    pub in_register: bool,
    pub signext: bool,
}

impl Debug for Extend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Extend")
            .field("offset_align", &self.offset_align)
            .field("coerce_to_type", &self.coerce_to_type.map(ShowLLVMType))
            .field("padding", &self.padding.map(ShowLLVMType))
            .field("in_register", &self.in_register)
            .field("signext", &self.signext)
            .finish()
    }
}

#[derive(Clone)]
pub struct Indirect {
    pub padding: Option<LLVMTypeRef>,
    pub align: ByteCount,
    pub byval: bool,
    pub realign: bool,
    pub sret_after_this: bool,
    pub in_register: bool,
}

impl Debug for Indirect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Indirect")
            .field("padding", &self.padding.map(ShowLLVMType))
            .field("align", &self.align)
            .field("byval", &self.byval)
            .field("realign", &self.realign)
            .field("sret_after_this", &self.sret_after_this)
            .field("in_register", &self.in_register)
            .finish()
    }
}

#[derive(Clone)]
pub struct IndirectAliased {
    pub padding: Option<LLVMTypeRef>,
    pub align: ByteCount,
    pub realign: bool,
    pub address_space: u32,
}

impl Debug for IndirectAliased {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IndirectAliased")
            .field("padding", &self.padding.map(ShowLLVMType))
            .field("align", &self.align)
            .field("realign", &self.realign)
            .field("address_space", &self.address_space)
            .finish()
    }
}

#[derive(Clone)]
pub struct Expand {
    pub padding: Option<LLVMTypeRef>,
}

impl Debug for Expand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Expand")
            .field("padding", &self.padding.map(ShowLLVMType))
            .finish()
    }
}

#[derive(Clone, Debug)]
pub struct InAlloca {
    pub alloca_field_index: u32,
    pub sret: bool,
    pub indirect: bool,
}

#[derive(Clone)]
pub struct CoerceAndExpand {
    pub coerce_to_type: LLVMTypeRef,
    pub unpadded_coerce_and_expand_type: LLVMTypeRef,
}

impl Debug for CoerceAndExpand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CoerceAndExpand")
            .field("coerce_to_type", &ShowLLVMType(self.coerce_to_type))
            .field(
                "unpadded_coerce_and_expand_type",
                &ShowLLVMType(self.unpadded_coerce_and_expand_type),
            )
            .finish()
    }
}
