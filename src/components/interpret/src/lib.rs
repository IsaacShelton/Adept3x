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
use crate::{registers::Registers, value::StructLiteral};
pub use error::InterpreterError;
use ir;
use std::collections::HashMap;
pub use value::Value;

#[derive(Debug)]
pub struct Interpreter<'a, S: SyscallHandler> {
    pub syscall_handler: S,
    max_steps_left: Option<u64>,
    ir_module: &'a ir::Module,
    memory: Memory,
    global_addresses: HashMap<ir::GlobalRef, ir::Literal>,
}

impl<'a, S: SyscallHandler> Interpreter<'a, S> {
    pub fn new(syscall_handler: S, ir_module: &'a ir::Module, max_steps_left: Option<u64>) -> Self {
        let mut memory = Memory::new();

        let mut global_addresses = HashMap::new();
        for (global_ref, global) in ir_module.globals.iter() {
            let address = memory.alloc_permanent(size_of(&global.ir_type, ir_module));
            global_addresses.insert(global_ref, address);
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
        interpreter_entry_point: ir::FuncRef,
    ) -> Result<Value<'a>, InterpreterError> {
        // The entry point for the interpreter always takes zero arguments
        // and can return anything. It is up to the producer of the ir::Module
        // to create an interpreter entry point that calls the actual function(s)
        // they care about, and return the value they want.
        self.call(interpreter_entry_point, vec![])
    }

    pub fn call(
        &mut self,
        func_ref: ir::FuncRef,
        args: Vec<Value<'a>>,
    ) -> Result<Value<'a>, InterpreterError> {
        let function = self.ir_module.funcs.get(func_ref);

        if function.is_cstyle_variadic {
            todo!(
                "c-style variadic functions are not supported in interpreter yet - (for function {:?})",
                function.mangled_name
            );
        }

        assert_eq!(function.params.len(), args.len());

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
                ir::Instr::Return(value) => {
                    break value;
                }
                ir::Instr::Call(call) => {
                    let mut arguments = Vec::with_capacity(call.args.len());

                    for argument in call.args.iter() {
                        arguments.push(self.eval(&registers, argument));
                    }

                    self.call(call.func, arguments)?
                }
                ir::Instr::Alloca(ty) => {
                    Value::Literal(self.memory.alloc_stack(self.size_of(&ty))?)
                }
                ir::Instr::Store(store) => {
                    let new_value = self.eval(&registers, &store.new_value);
                    let dest = self.eval(&registers, &store.destination).as_u64().unwrap();

                    self.memory.write(dest, new_value, self.ir_module)?;
                    Value::Undefined
                }
                ir::Instr::Load((value, ty)) => {
                    let address = self.eval(&registers, &value).as_u64().unwrap();
                    self.memory.read(address, ty)?
                }
                ir::Instr::Malloc(ir_type) => {
                    let bytes = self.size_of(ir_type);
                    Value::Literal(self.memory.alloc_heap(bytes))
                }
                ir::Instr::MallocArray(ir_type, count) => {
                    let count = self.eval(&registers, count);
                    let count = count.as_u64().unwrap();
                    let bytes = self.size_of(ir_type);
                    Value::Literal(self.memory.alloc_heap(bytes * count))
                }
                ir::Instr::Free(value) => {
                    let value = self.eval(&registers, value).unwrap_literal();
                    self.memory.free_heap(value);
                    Value::Literal(ir::Literal::Void)
                }
                ir::Instr::SizeOf(ty) => Value::Literal(ir::Literal::Unsigned64(self.size_of(ty))),
                ir::Instr::Parameter(index) => args[usize::try_from(*index).unwrap()].clone(),
                ir::Instr::GlobalVariable(global_ref) => {
                    Value::Literal(self.global_addresses.get(global_ref).unwrap().clone())
                }
                ir::Instr::Add(ops, _f_or_i) => self.add(ops, &registers),
                ir::Instr::Checked(_, _) => todo!(),
                ir::Instr::Subtract(ops, _f_or_i) => self.sub(ops, &registers),
                ir::Instr::Multiply(ops, _f_or_i) => self.mul(ops, &registers),
                ir::Instr::Divide(ops, _f_or_sign) => self.div(ops, &registers)?,
                ir::Instr::Modulus(ops, _f_or_sign) => self.rem(ops, &registers)?,
                ir::Instr::Equals(ops, _f_or_i) => self.eq(ops, &registers),
                ir::Instr::NotEquals(ops, _f_or_i) => self.neq(ops, &registers),
                ir::Instr::LessThan(ops, _f_or_i) => self.lt(ops, &registers),
                ir::Instr::LessThanEq(ops, _f_or_i) => self.lte(ops, &registers),
                ir::Instr::GreaterThan(ops, _f_or_i) => self.gt(ops, &registers),
                ir::Instr::GreaterThanEq(ops, _f_or_i) => self.gte(ops, &registers),
                ir::Instr::And(_) => todo!("Interpreter / ir::Instruction::And"),
                ir::Instr::Or(_) => todo!("Interpreter / ir::Instruction::Or"),
                ir::Instr::BitwiseAnd(_) => {
                    todo!("Interpreter / ir::Instruction::BitwiseAnd")
                }
                ir::Instr::BitwiseOr(_) => todo!("Interpreter / ir::Instruction::BitwiseOr"),
                ir::Instr::BitwiseXor(_) => {
                    todo!("Interpreter / ir::Instruction::BitwiseXor")
                }
                ir::Instr::LeftShift(_) => todo!("Interpreter / ir::Instruction::LeftShift"),
                ir::Instr::ArithmeticRightShift(_) => {
                    todo!("Interpreter / ir::Instruction::ArithmeticRightShift")
                }
                ir::Instr::LogicalRightShift(_) => {
                    todo!("Interpreter / ir::Instruction::LogicalRightShift")
                }
                ir::Instr::Bitcast(_, _) => todo!("Interpreter / ir::Instruction::BitCast"),
                ir::Instr::ZeroExtend(_, _) => {
                    todo!("Interpreter / ir::Instruction::ZeroExtend")
                }
                ir::Instr::SignExtend(_, _) => {
                    todo!("Interpreter / ir::Instruction::SignExtend")
                }
                ir::Instr::FloatExtend(_, _) => {
                    todo!("Interpreter / ir::Instruction::FloatExtend")
                }
                ir::Instr::Truncate(_, _) => todo!("Interpreter / ir::Instruction::Truncate"),
                ir::Instr::TruncateFloat(_, _) => {
                    todo!("Interpreter / ir::Instruction::TruncateFloat")
                }
                ir::Instr::IntegerToPointer(..) => {
                    todo!("Interpreter / ir::Instruction::IntegerToPointer");
                }
                ir::Instr::PointerToInteger(..) => {
                    todo!("Interpreter / ir::Instruction::PointerToInteger");
                }

                ir::Instr::FloatToInteger(..) => {
                    todo!("Interpreter / ir::Instruction::FloatToInteger");
                }
                ir::Instr::IntegerToFloat(..) => {
                    todo!("Interpreter / ir::Instruction::IntegerToFloat");
                }
                ir::Instr::Member {
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
                ir::Instr::ArrayAccess { .. } => {
                    todo!("Interpreter / ir::Instruction::ArrayAccess")
                }
                ir::Instr::StructLiteral(ty, values) => {
                    let mut field_values = Vec::with_capacity(values.len());

                    for value in values {
                        field_values.push(self.eval(&registers, value));
                    }

                    Value::StructLiteral(StructLiteral {
                        values: field_values,
                        fields: ty.struct_fields(self.ir_module).unwrap(),
                    })
                }
                ir::Instr::IsZero(_value, _) => {
                    todo!("Interpreter / ir::Instruction::IsZero")
                }
                ir::Instr::IsNonZero(value, _) => {
                    let value = self.eval(&registers, value);

                    match value {
                        Value::Undefined => Value::Undefined,
                        Value::Literal(literal) => {
                            Value::Literal(ir::Literal::Boolean(match literal {
                                ir::Literal::Void => false,
                                ir::Literal::Boolean(x) => x,
                                ir::Literal::Signed8(x) => x != 0,
                                ir::Literal::Signed16(x) => x != 0,
                                ir::Literal::Signed32(x) => x != 0,
                                ir::Literal::Signed64(x) => x != 0,
                                ir::Literal::Unsigned8(x) => x != 0,
                                ir::Literal::Unsigned16(x) => x != 0,
                                ir::Literal::Unsigned32(x) => x != 0,
                                ir::Literal::Unsigned64(x) => x != 0,
                                ir::Literal::Float32(x) => x != 0.0,
                                ir::Literal::Float64(x) => x != 0.0,
                                ir::Literal::NullTerminatedString(_) => true,
                                ir::Literal::Zeroed(_) => false,
                            }))
                        }
                        Value::StructLiteral(_) => Value::Undefined,
                    }
                }
                ir::Instr::Negate(..) => todo!("Interpreter / ir::Instruction::Negate"),
                ir::Instr::BitComplement(_) => {
                    todo!("Interpreter / ir::Instruction::BitComplement")
                }
                ir::Instr::Break(break_info) => {
                    new_ip = Some(InstructionPointer {
                        basicblock_id: break_info.basicblock_id,
                        instruction_id: 0,
                    });
                    Value::Undefined
                }
                ir::Instr::ConditionalBreak(value, break_info) => {
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
                ir::Instr::Phi(phi) => {
                    let mut found = None;

                    for incoming in phi.incoming.iter() {
                        if incoming.basicblock_id == came_from_block {
                            found = Some(self.eval(&registers, &incoming.value));
                            break;
                        }
                    }

                    found.unwrap_or(Value::Undefined)
                }
                ir::Instr::InterpreterSyscall(syscall, supplied_args) => {
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
