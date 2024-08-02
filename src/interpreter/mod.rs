mod error;
mod ip;
mod memory;
mod ops;
mod registers;
mod size_of;
pub mod syscall_handler;
mod value;

use self::{
    ip::InstructionPointer, memory::Memory, size_of::size_of, syscall_handler::SyscallHandler,
};
use crate::{
    interpreter::{registers::Registers, value::StructLiteral},
    ir,
};
pub use error::InterpreterError;
use std::collections::HashMap;
pub use value::Value;

#[derive(Debug)]
pub struct Interpreter<'a, S: SyscallHandler> {
    pub syscall_handler: S,
    max_steps_left: Option<u64>,
    ir_module: &'a ir::Module<'a>,
    memory: Memory,
    global_addresses: HashMap<ir::GlobalVarRef, ir::Literal>,
}

impl<'a, S: SyscallHandler> Interpreter<'a, S> {
    pub fn new(syscall_handler: S, ir_module: &'a ir::Module, max_steps_left: Option<u64>) -> Self {
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
            syscall_handler,
        }
    }

    pub fn run(
        &mut self,
        interpreter_entry_point: ir::FunctionRef,
    ) -> Result<Value<'a>, InterpreterError> {
        // The entry point for the interpreter always takes zero arguments
        // and can return anything. It is up to the producer of the ir::Module
        // to create an interpreter entry point that calls the actual function(s)
        // they care about, and return the value they want.
        self.call(interpreter_entry_point, vec![])
    }

    pub fn call(
        &mut self,
        function_ref: ir::FunctionRef,
        args: Vec<Value<'a>>,
    ) -> Result<Value<'a>, InterpreterError> {
        let function = self.ir_module.functions.get(&function_ref).unwrap();

        if function.is_cstyle_variadic {
            todo!("c-style variadic functions are not supported in interpreter yet");
        }

        assert_eq!(function.parameters.len(), args.len());

        let mut registers = Registers::<'a>::new(&function.basicblocks);
        let mut ip = InstructionPointer::default();

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
                ir::Instruction::Call(call) => {
                    let mut arguments = Vec::with_capacity(call.arguments.len());

                    for argument in call.arguments.iter() {
                        arguments.push(self.eval(&registers, argument));
                    }

                    self.call(call.function, arguments)?
                }
                ir::Instruction::Alloca(ty) => {
                    Value::Literal(self.memory.alloc_stack(self.size_of(ty))?)
                }
                ir::Instruction::Store(store) => {
                    let new_value = self.eval(&registers, &store.new_value);
                    let dest = self.eval(&registers, &store.destination).as_u64().unwrap();

                    self.memory.write(dest, new_value, self.ir_module)?;
                    Value::Undefined
                }
                ir::Instruction::Load((value, ty)) => {
                    let address = self.eval(&registers, value).as_u64().unwrap();
                    self.memory.read(address, ty)?
                }
                ir::Instruction::Malloc(ir_type) => {
                    let bytes = self.size_of(ir_type);
                    Value::Literal(self.memory.alloc_heap(bytes))
                }
                ir::Instruction::MallocArray(ir_type, count) => {
                    let count = self.eval(&registers, count);
                    let count = count.as_u64().unwrap();
                    let bytes = self.size_of(ir_type);
                    Value::Literal(self.memory.alloc_heap(bytes * count))
                }
                ir::Instruction::Free(value) => {
                    let value = self.eval(&registers, value).unwrap_literal();
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
                ir::Instruction::Add(ops, _f_or_i) => self.add(ops, &registers),
                ir::Instruction::Checked(_, _) => todo!(),
                ir::Instruction::Subtract(ops, _f_or_i) => self.sub(ops, &registers),
                ir::Instruction::Multiply(ops, _f_or_i) => self.mul(ops, &registers),
                ir::Instruction::Divide(ops, _f_or_sign) => self.div(ops, &registers)?,
                ir::Instruction::Modulus(ops, _f_or_sign) => self.rem(ops, &registers)?,
                ir::Instruction::Equals(ops, _f_or_i) => self.eq(ops, &registers),
                ir::Instruction::NotEquals(ops, _f_or_i) => self.neq(ops, &registers),
                ir::Instruction::LessThan(ops, _f_or_i) => self.lt(ops, &registers),
                ir::Instruction::LessThanEq(ops, _f_or_i) => self.lte(ops, &registers),
                ir::Instruction::GreaterThan(ops, _f_or_i) => self.gt(ops, &registers),
                ir::Instruction::GreaterThanEq(ops, _f_or_i) => self.gte(ops, &registers),
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
                ir::Instruction::Member {
                    struct_type,
                    subject_pointer,
                    index,
                } => {
                    let fields = struct_type.struct_fields(self.ir_module).unwrap();

                    let offset = fields
                        .iter()
                        .take(*index)
                        .fold(0, |acc, f| acc + self.size_of(&f.ir_type));

                    let subject_pointer = self.eval(&registers, subject_pointer).as_u64().unwrap();
                    Value::Literal(ir::Literal::Unsigned64(subject_pointer + offset))
                }
                ir::Instruction::ArrayAccess { .. } => todo!(),
                ir::Instruction::StructureLiteral(ty, values) => {
                    let mut field_values = Vec::with_capacity(values.len());

                    for value in values {
                        field_values.push(self.eval(&registers, value));
                    }

                    Value::StructLiteral(StructLiteral {
                        values: field_values,
                        fields: ty.struct_fields(self.ir_module).unwrap(),
                    })
                }
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
                    let value = self.eval(&registers, value);

                    let should = match value {
                        Value::Literal(ir::Literal::Boolean(value)) => value,
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
                            found = Some(self.eval(&registers, &incoming.value));
                            break;
                        }
                    }

                    found.unwrap_or(Value::Undefined)
                }
                ir::Instruction::InterpreterSyscall(syscall, supplied_args) => {
                    let mut args = Vec::with_capacity(args.len());

                    for supplied_arg in supplied_args.iter() {
                        args.push(self.eval(&registers, supplied_arg));
                    }

                    self.syscall_handler
                        .syscall(&mut self.memory, *syscall, args)
                }
            };

            registers.set(&ip, result);

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
            .map(|value| self.eval(&registers, value))
            .unwrap_or(Value::Literal(ir::Literal::Void)))
    }

    pub fn eval(&self, registers: &Registers<'a>, value: &ir::Value) -> Value<'a> {
        match value {
            ir::Value::Literal(literal) => Value::Literal(literal.clone()),
            ir::Value::Reference(reference) => registers.get(reference).clone(),
        }
    }

    pub fn size_of(&self, ir_type: &ir::Type) -> u64 {
        size_of(ir_type, self.ir_module)
    }
}
