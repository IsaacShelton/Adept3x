mod error;
mod ip;
mod memory;
mod ops;
mod size_of;
mod value;

use self::{
    error::InterpreterError, ip::InstructionPointer, memory::Memory, size_of::size_of, value::Value,
};
use crate::ir;
use std::collections::HashMap;

#[derive(Debug)]
pub struct Interpreter<'a> {
    max_steps_left: Option<u64>,
    ir_module: &'a ir::Module,
    memory: Memory,
    global_addresses: HashMap<ir::GlobalRef, ir::Literal>,
}

impl<'a> Interpreter<'a> {
    pub fn new(ir_module: &'a ir::Module, max_steps_left: Option<u64>) -> Self {
        let mut memory = Memory::new();

        let mut global_addresses = HashMap::new();
        for (global_ref, global) in ir_module.globals.iter() {
            let address = memory.alloc_permanent(size_of(&global.ir_type, ir_module));
            global_addresses.insert(*global_ref, address);
        }

        Self {
            max_steps_left,
            ir_module,
            memory,
            global_addresses,
        }
    }

    pub fn start_main(&mut self, main_fn_name: &str) -> Result<Value, InterpreterError> {
        // TODO: We probably should've cached this so we don't have to do
        // this slow search here
        let main_function = match self
            .ir_module
            .functions
            .iter()
            .find(|(_, f)| f.mangled_name == main_fn_name)
        {
            Some((f_ref, _)) => *f_ref,
            None => return Err(InterpreterError::MissingMainFunction),
        };

        self.call(main_function, vec![])
    }

    pub fn call(
        &mut self,
        function_ref: ir::FunctionRef,
        args: Vec<Value>,
    ) -> Result<Value, InterpreterError> {
        let function = self.ir_module.functions.get(&function_ref).unwrap();

        if function.is_cstyle_variadic {
            todo!();
        }

        assert_eq!(function.parameters.len(), args.len());

        let mut block_registers = Vec::new();
        for block in function.basicblocks.iter() {
            let registers =
                Vec::from_iter(std::iter::repeat(Value::Undefined).take(block.instructions.len()));
            block_registers.push(registers);
        }

        let mut ip = InstructionPointer {
            basicblock_id: 0,
            instruction_id: 0,
        };

        let fp = self.memory.stack_save();
        let mut came_from_block = 0;

        let return_value = loop {
            if let Some(max_steps) = &mut self.max_steps_left {
                if *max_steps == 0 {
                    return Err(InterpreterError::TimedOut);
                }

                *max_steps -= 1;
            }

            let instruction =
                &function.basicblocks.blocks[ip.basicblock_id].instructions[ip.instruction_id];
            let mut new_ip = None;

            let result = match instruction {
                ir::Instruction::Return(value) => {
                    break value;
                }
                ir::Instruction::Call(_) => todo!(),
                ir::Instruction::Alloca(ty) => {
                    Value::Literal(self.memory.alloc_stack(self.size_of(ty))?)
                }
                ir::Instruction::Store(store) => {
                    let new_value = self.eval(&block_registers, &store.new_value);
                    let destination = self
                        .eval(&block_registers, &store.destination)
                        .as_u64()
                        .unwrap();

                    self.memory.write(destination, new_value, self.ir_module)?;
                    Value::Undefined
                }
                ir::Instruction::Load((value, ty)) => {
                    let address = self.eval(&block_registers, value).as_u64().unwrap();
                    self.memory.read(address, ty, self.ir_module)?
                }
                ir::Instruction::Malloc(ir_type) => {
                    let bytes = self.size_of(ir_type);
                    Value::Literal(self.memory.alloc_heap(bytes))
                }
                ir::Instruction::MallocArray(ir_type, count) => {
                    let count = self.eval(&block_registers, count);
                    let count = count.as_u64().unwrap();
                    let bytes = self.size_of(ir_type);
                    Value::Literal(self.memory.alloc_heap(bytes * count))
                }
                ir::Instruction::Free(value) => {
                    let value = self.eval(&block_registers, value).unwrap_literal();
                    self.memory.free_heap(value);
                    Value::Literal(ir::Literal::Void)
                }
                ir::Instruction::SizeOf(ty) => {
                    Value::Literal(ir::Literal::Unsigned64(self.size_of(ty)))
                }
                ir::Instruction::Parameter(index) => args[usize::try_from(*index).unwrap()].clone(),
                ir::Instruction::GlobalVariable(global_ref) => {
                    Value::Literal(self.global_addresses.get(global_ref).unwrap().clone())
                }
                ir::Instruction::Add(operands, _float_or_int) => {
                    self.add(&operands, &block_registers)
                }
                ir::Instruction::Checked(_, _) => todo!(),
                ir::Instruction::Subtract(operands, _float_or_int) => {
                    self.sub(&operands, &block_registers)
                }
                ir::Instruction::Multiply(operands, _float_or_int) => {
                    self.mul(&operands, &block_registers)
                }
                ir::Instruction::Divide(operands, _float_or_sign) => {
                    self.div(&operands, &block_registers)?
                }
                ir::Instruction::Modulus(operands, _float_or_sign) => {
                    self.rem(&operands, &block_registers)?
                }
                ir::Instruction::Equals(_, _) => todo!(),
                ir::Instruction::NotEquals(_, _) => todo!(),
                ir::Instruction::LessThan(_, _) => todo!(),
                ir::Instruction::LessThanEq(_, _) => todo!(),
                ir::Instruction::GreaterThan(_, _) => todo!(),
                ir::Instruction::GreaterThanEq(_, _) => todo!(),
                ir::Instruction::And(_) => todo!(),
                ir::Instruction::Or(_) => todo!(),
                ir::Instruction::BitwiseAnd(_) => todo!(),
                ir::Instruction::BitwiseOr(_) => todo!(),
                ir::Instruction::BitwiseXor(_) => todo!(),
                ir::Instruction::LeftShift(_) => todo!(),
                ir::Instruction::RightShift(_) => todo!(),
                ir::Instruction::LogicalRightShift(_) => todo!(),
                ir::Instruction::Bitcast(_, _) => todo!(),
                ir::Instruction::ZeroExtend(_, _) => todo!(),
                ir::Instruction::SignExtend(_, _) => todo!(),
                ir::Instruction::FloatExtend(_, _) => todo!(),
                ir::Instruction::Truncate(_, _) => todo!(),
                ir::Instruction::TruncateFloat(_, _) => todo!(),
                ir::Instruction::Member { .. } => todo!(),
                ir::Instruction::ArrayAccess { .. } => todo!(),
                ir::Instruction::StructureLiteral(_, _) => todo!(),
                ir::Instruction::IsZero(_) => todo!(),
                ir::Instruction::IsNotZero(_) => todo!(),
                ir::Instruction::Negate(_) => todo!(),
                ir::Instruction::NegateFloat(_) => todo!(),
                ir::Instruction::BitComplement(_) => todo!(),
                ir::Instruction::Break(break_info) => {
                    new_ip = Some(InstructionPointer {
                        basicblock_id: break_info.basicblock_id,
                        instruction_id: 0,
                    });
                    Value::Undefined
                }
                ir::Instruction::ConditionalBreak(value, break_info) => {
                    let value = self.eval(&block_registers, value);

                    let should = match value {
                        Value::Literal(literal) => match literal {
                            ir::Literal::Boolean(value) => value,
                            _ => false,
                        },
                        _ => false,
                    };

                    new_ip = Some(InstructionPointer {
                        basicblock_id: if should {
                            break_info.true_basicblock_id
                        } else {
                            break_info.false_basicblock_id
                        },
                        instruction_id: 0,
                    });

                    Value::Undefined
                }
                ir::Instruction::Phi(phi) => {
                    let mut found = None;

                    for incoming in phi.incoming.iter() {
                        if incoming.basicblock_id == came_from_block {
                            found = Some(self.eval(&block_registers, &incoming.value));
                            break;
                        }
                    }

                    found.unwrap_or(Value::Undefined)
                }
            };

            block_registers[ip.basicblock_id][ip.instruction_id] = result;

            if new_ip.is_some() {
                came_from_block = ip.basicblock_id;
            }

            ip = new_ip.unwrap_or_else(|| InstructionPointer {
                instruction_id: ip.instruction_id + 1,
                ..ip
            });
        };

        self.memory.stack_restore(fp);
        Ok(return_value
            .as_ref()
            .map(|value| self.eval(&block_registers, value))
            .unwrap_or(Value::Literal(ir::Literal::Void)))
    }

    pub fn eval(&self, block_registers: &Vec<Vec<Value>>, value: &ir::Value) -> Value {
        match value {
            ir::Value::Literal(literal) => Value::Literal(literal.clone()),
            ir::Value::Reference(reference) => {
                block_registers[reference.basicblock_id][reference.instruction_id].clone()
            }
        }
    }

    pub fn size_of(&self, ir_type: &ir::Type) -> u64 {
        size_of(ir_type, self.ir_module)
    }
}
