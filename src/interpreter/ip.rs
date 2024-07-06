#[derive(Clone, Debug, Default)]
pub struct InstructionPointer {
    pub basicblock_id: usize,
    pub instruction_id: usize,
}
