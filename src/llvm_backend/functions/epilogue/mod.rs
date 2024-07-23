use crate::llvm_backend::builder::Builder;
use llvm_sys::prelude::LLVMBasicBlockRef;

pub struct EpilogueInfo {
    pub llvm_basicblock: LLVMBasicBlockRef,
}

pub fn emit_epilogue(builder: &Builder, llvm_basicblock: LLVMBasicBlockRef) -> EpilogueInfo {
    builder.position(llvm_basicblock);

    EpilogueInfo { llvm_basicblock }
}
