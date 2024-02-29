use llvm_sys::prelude::LLVMValueRef;

pub struct ValueCatalog {
    blocks: Vec<Block>,
}

struct Block {
    values: Vec<LLVMValueRef>,
}
