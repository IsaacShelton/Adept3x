#[derive(Clone, Debug, Default)]
pub struct InstructionPointer {
    pub basicblock_id: usize,
    pub instruction_id: usize,
}

impl InstructionPointer {
    pub fn increment(&self) -> Self {
        Self {
            basicblock_id: self.basicblock_id,
            instruction_id: self.instruction_id + 1,
        }
    }
}
