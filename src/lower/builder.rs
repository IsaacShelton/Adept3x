use crate::ir::{BasicBlock, BasicBlocks, Instruction};

pub struct Builder {
    pub basicblocks: BasicBlocks,
}

impl Builder {
    pub fn new() -> Self {
        Self {
            basicblocks: BasicBlocks::new(),
        }
    }

    pub fn build(self) -> BasicBlocks {
        self.basicblocks
    }

    pub fn is_terminated(&self) -> bool {
        self.basicblocks.is_terminated()
    }

    pub fn terminate(&mut self) {
        if !self.is_terminated() {
            self.push(Instruction::Return(None));
        }
    }

    pub fn push(&mut self, instruction: Instruction) {
        if let Some(last) = self.basicblocks.last_mut() {
            last.push(instruction);
        } else {
            let mut first_block = BasicBlock::new();
            first_block.push(instruction);
            self.basicblocks.push(first_block);
        }
    }
}
