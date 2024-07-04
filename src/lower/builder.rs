use crate::ir::{self, BasicBlock, BasicBlocks, Instruction, ValueReference};

pub struct Builder {
    basicblocks: BasicBlocks,
    current_basicblock_id: usize,
}

impl Builder {
    pub fn new_with_starting_block() -> Self {
        let mut basicblocks = BasicBlocks::new();
        basicblocks.push(BasicBlock::new());

        Self {
            basicblocks,
            current_basicblock_id: 0,
        }
    }

    pub fn build(self) -> BasicBlocks {
        self.basicblocks
    }

    pub fn is_block_terminated(&self) -> bool {
        self.basicblocks.len() > 0
            && self.basicblocks.blocks[self.current_basicblock_id].is_terminated()
    }

    pub fn continues_to(&mut self, basicblock_id: usize) {
        if !self.is_block_terminated() {
            self.push(ir::Instruction::Break(ir::Break { basicblock_id }));
        }
    }

    pub fn terminate(&mut self) {
        if !self.is_block_terminated() {
            self.push(Instruction::Return(None));
        }
    }

    pub fn new_block(&mut self) -> usize {
        let block = BasicBlock::new();
        let id = self.basicblocks.len();
        self.basicblocks.push(block);
        id
    }

    pub fn use_block(&mut self, id: usize) {
        if id >= self.basicblocks.len() {
            panic!("attempt to build with basicblock that doesn't exist");
        }

        self.current_basicblock_id = id;
    }

    pub fn current_block_id(&mut self) -> usize {
        if self.basicblocks.len() == 0 {
            self.basicblocks.push(BasicBlock::new());
            0
        } else {
            self.current_basicblock_id
        }
    }

    pub fn push(&mut self, instruction: Instruction) -> ir::Value {
        let current_block = self
            .basicblocks
            .get_mut(self.current_basicblock_id)
            .expect("at least one basicblock");

        current_block.push(instruction);

        ir::Value::Reference(ValueReference {
            basicblock_id: self.current_basicblock_id,
            instruction_id: current_block.instructions.len() - 1,
        })
    }
}
