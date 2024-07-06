use super::{ip::InstructionPointer, Value};
use crate::ir::{BasicBlocks, ValueReference};

#[derive(Debug)]
pub struct Registers {
    registers: Vec<Vec<Value>>,
}

impl Registers {
    pub fn new(basicblocks: &BasicBlocks) -> Self {
        let mut registers = Vec::with_capacity(basicblocks.len());

        for block in basicblocks.iter() {
            registers.push(Vec::from_iter(
                std::iter::repeat(Value::Undefined).take(block.instructions.len()),
            ));
        }

        Self { registers }
    }

    pub fn set(&mut self, ip: &InstructionPointer, value: Value) {
        self.set_raw(ip.basicblock_id, ip.instruction_id, value)
    }

    pub fn set_raw(&mut self, block: usize, instruction: usize, value: Value) {
        self.registers[block][instruction] = value;
    }

    pub fn get(&self, reference: &ValueReference) -> &Value {
        self.get_raw(reference.basicblock_id, reference.instruction_id)
    }

    pub fn get_raw(&self, block: usize, instruction: usize) -> &Value {
        &self.registers[block][instruction]
    }
}
