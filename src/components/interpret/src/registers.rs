use super::{Value, ip::InstructionPointer};
use crate::{
    ir::{BasicBlocks, ValueReference},
    value::ValueKind,
};

#[derive(Debug)]
pub struct Registers<'a> {
    registers: Vec<Vec<Value<'a>>>,
}

impl<'a> Registers<'a> {
    pub fn new(basicblocks: &BasicBlocks) -> Self {
        let mut registers = Vec::with_capacity(basicblocks.len());

        for block in basicblocks.iter() {
            registers.push(Vec::from_iter(
                std::iter::repeat(ValueKind::Undefined.untainted()).take(block.instructions.len()),
            ));
        }

        Self { registers }
    }

    pub fn set(&mut self, ip: &InstructionPointer, value: Value<'a>) {
        self.set_raw(ip.basicblock_id, ip.instruction_id, value)
    }

    pub fn set_raw(&mut self, block: usize, instruction: usize, value: Value<'a>) {
        self.registers[block][instruction] = value;
    }

    pub fn get(&self, reference: &ValueReference) -> &Value<'a> {
        self.get_raw(reference.basicblock_id, reference.instruction_id)
    }

    pub fn get_raw(&self, block: usize, instruction: usize) -> &Value<'a> {
        &self.registers[block][instruction]
    }
}
