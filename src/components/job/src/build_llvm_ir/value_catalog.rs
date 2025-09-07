use crate::ir::ValueReference;
use llvm_sys::prelude::LLVMValueRef;

#[derive(Clone, Debug)]
pub struct ValueCatalog {
    blocks: Vec<Block>,
}

#[derive(Clone, Debug, Default)]
struct Block {
    values: Vec<Option<LLVMValueRef>>,
}

impl ValueCatalog {
    pub fn new(num_blocks: usize) -> Self {
        Self {
            blocks: vec![Default::default(); num_blocks],
        }
    }

    pub fn get(&self, reference: &ValueReference) -> Option<LLVMValueRef> {
        self.blocks.get(reference.basicblock_id).and_then(|block| {
            block
                .values
                .get(reference.instruction_id)
                .and_then(|value| *value)
        })
    }

    pub fn push(&mut self, basicblock_id: usize, value: Option<LLVMValueRef>) {
        self.blocks[basicblock_id].values.push(value);
    }
}
