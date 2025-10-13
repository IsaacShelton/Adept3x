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

    pub fn get(&self, reference: &ValueReference) -> Result<LLVMValueRef, ValueReferenceError> {
        let block = self.blocks.get(reference.basicblock_id).ok_or_else(|| {
            ValueReferenceError::BasicBlockDoesNotExist {
                count: self.blocks.len(),
                got: reference.basicblock_id,
            }
        })?;

        let instr = block.values.get(reference.instruction_id).ok_or_else(|| {
            ValueReferenceError::InstructionDoesNotExist {
                count: block.values.len(),
                got: reference.instruction_id,
            }
        })?;

        let value = instr.ok_or_else(|| ValueReferenceError::InstructionNotLoweredYet {
            reference: *reference,
        })?;

        Ok(value)
    }

    pub fn push(&mut self, basicblock_id: usize, value: Option<LLVMValueRef>) {
        self.blocks[basicblock_id].values.push(value);
    }
}

#[derive(Debug)]
pub enum ValueReferenceError {
    #[allow(unused)]
    BasicBlockDoesNotExist { count: usize, got: usize },
    #[allow(unused)]
    InstructionDoesNotExist { count: usize, got: usize },
    #[allow(unused)]
    InstructionNotLoweredYet { reference: ValueReference },
}
