use super::{Value, ip::InstructionPointer};
use crate::{
    interpret::value::ValueKind,
    ir::{self, ValueReference},
};

#[derive(Debug)]
pub struct Registers<'env> {
    registers: Vec<Vec<Value<'env>>>,
}

impl<'env> Registers<'env> {
    pub fn new(basicblocks: &'env [ir::BasicBlock<'env>]) -> Self {
        let mut registers = Vec::with_capacity(basicblocks.len());

        for block in basicblocks.iter() {
            registers.push(Vec::from_iter(
                std::iter::repeat(ValueKind::Undefined.untainted()).take(block.instructions.len()),
            ));
        }

        Self { registers }
    }

    pub fn set(&mut self, ip: &InstructionPointer, value: Value<'env>) {
        self.set_raw(ip.basicblock_id, ip.instruction_id, value)
    }

    pub fn set_raw(&mut self, block: usize, instruction: usize, value: Value<'env>) {
        self.registers[block][instruction] = value;
    }

    pub fn get(&self, reference: &ValueReference) -> &Value<'env> {
        self.get_raw(reference.basicblock_id, reference.instruction_id)
    }

    pub fn get_raw(&self, block: usize, instruction: usize) -> &Value<'env> {
        &self.registers[block][instruction]
    }
}
