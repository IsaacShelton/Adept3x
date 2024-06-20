use llvm_sys::prelude::LLVMTypeRef;

#[derive(Copy, Clone, Debug)]
pub enum ABIPassAttribute {
    ZeroExtend,
    StructReturn,
}

#[derive(Copy, Clone, Debug)]
pub enum ABIPassKind {
    // Pass argument by normal or coerced type
    Direct,
    // Pass argument by pointer
    Indirect,
    // Ignore the argument (used for empty struct types)
    Ignore,
}

#[derive(Clone, Debug)]
pub struct ABIType {
    pub kind: ABIPassKind,
    pub original_type: LLVMTypeRef,
    pub coerced_type: Option<LLVMTypeRef>,
    pub prefix_with_padding: Option<LLVMTypeRef>,
    pub attribute: Option<ABIPassAttribute>,
}

impl ABIType {
    pub fn new_direct(
        original_type: LLVMTypeRef,
        coerced_type: Option<LLVMTypeRef>,
        prefix_with_padding: Option<LLVMTypeRef>,
        attribute: Option<ABIPassAttribute>,
    ) -> Self {
        Self {
            kind: ABIPassKind::Direct,
            original_type,
            coerced_type,
            prefix_with_padding,
            attribute,
        }
    }

    pub fn new_indirect(original_type: LLVMTypeRef, attribute: Option<ABIPassAttribute>) -> Self {
        Self {
            kind: ABIPassKind::Indirect,
            original_type,
            coerced_type: None,
            prefix_with_padding: None,
            attribute,
        }
    }

    pub fn new_ignored(original_type: LLVMTypeRef) -> Self {
        Self {
            kind: ABIPassKind::Ignore,
            original_type,
            coerced_type: None,
            prefix_with_padding: None,
            attribute: None,
        }
    }
}
