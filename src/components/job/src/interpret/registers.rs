use super::{Value, ip::InstructionPointer};
use crate::{
    interpret::value::{Tainted, ValueKind},
    ir::{self, BinaryOperands, ValueReference},
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

    pub fn eval(self: &Registers<'env>, value: &ir::Value<'env>) -> Value<'env> {
        match value {
            ir::Value::Literal(literal) => ValueKind::Literal(*literal).untainted(),
            ir::Value::Reference(reference) => self.get(reference).clone(),
        }
    }

    pub fn eval_into_literal(
        &self,
        value: &ir::Value<'env>,
    ) -> (ir::Literal<'env>, Option<Tainted>) {
        let reg = self.eval(value);
        (reg.kind.unwrap_literal(), reg.tainted)
    }

    pub fn eval_binary_ops(
        &self,
        operands: &BinaryOperands<'env>,
    ) -> (ir::Literal<'env>, ir::Literal<'env>, Option<Tainted>) {
        let (left, l_tainted) = self.eval_into_literal(&operands.left);
        let (right, r_tainted) = self.eval_into_literal(&operands.right);
        (left, right, l_tainted.or(r_tainted))
    }
}
