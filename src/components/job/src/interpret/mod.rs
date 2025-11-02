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
    interpret::{registers::Registers, value::StructLiteral},
    ir::{self, BinOp, BinOpFloatOrInteger, IntegerImmediate},
};
use ast::SizeOfMode;
use data_units::ByteUnits;
pub use error::InterpreterError;
use primitives::{IntegerBits, IntegerConstant};
use std::collections::HashMap;
pub use value::Value;
use value::{Tainted, ValueKind};

#[derive(Debug)]
pub struct Interpreter<'env, S: SyscallHandler> {
    pub syscall_handler: S,
    max_steps_left: Option<u64>,
    ir_module: &'env ir::Ir<'env>,
    memory: Memory,
    global_addresses: HashMap<ir::GlobalRef<'env>, ir::Literal<'env>>,
    exit_value: Option<u64>,
}

impl<'env, S: SyscallHandler> Interpreter<'env, S> {
    pub fn new(
        syscall_handler: S,
        ir_module: &'env ir::Ir<'env>,
        max_steps_left: Option<u64>,
    ) -> Self {
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
            exit_value: None,
        }
    }

    pub fn run(
        &mut self,
        interpreter_entry_point: ir::FuncRef<'env>,
    ) -> Result<Value<'env>, InterpreterError> {
        // The entry point for the interpreter always takes zero arguments
        // and can return anything. It is up to the producer of the ir::Module
        // to create an interpreter entry point that calls the actual function(s)
        // they care about, and return the value they want.
        self.call(interpreter_entry_point, vec![])
    }

    pub fn exit_value(&self) -> Option<u64> {
        self.exit_value
    }

    pub fn call<'a>(
        &mut self,
        func_ref: ir::FuncRef<'env>,
        args: Vec<Value<'env>>,
    ) -> Result<Value<'env>, InterpreterError> {
        let function = &self.ir_module.funcs[func_ref];

        if function.ownership.is_reference() {
            return Err(InterpreterError::CannotCallForeignFunction(
                function.mangled_name.to_string(),
            ));
        }

        if function.is_cstyle_variadic {
            todo!(
                "c-style variadic functions are not supported in interpreter yet - (for function {:?})",
                function.mangled_name
            );
        }

        assert_eq!(function.params.len(), args.len());

        let mut registers = Registers::<'env>::new(
            &function
                .basicblocks
                .get()
                .expect("callee to have body lowered")[..],
        );

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

            let instruction = &function.basicblocks.get().unwrap()[ip.basicblock_id].instructions
                [ip.instruction_id];
            let mut new_ip = None;

            let result = match instruction {
                ir::Instr::ExitInterpreter(argument) => {
                    self.exit_value = self.eval(&registers, argument).as_u64();
                    break Some(ir::Value::Literal(ir::Literal::Void));
                }
                ir::Instr::Return(value) => {
                    break *value;
                }
                ir::Instr::Call(call) => {
                    let mut arguments = Vec::with_capacity(call.args.len());

                    for argument in call.args.iter() {
                        arguments.push(self.eval(&registers, argument));
                    }

                    self.call(call.func, arguments)?
                }
                ir::Instr::Alloca(ty) => {
                    ValueKind::Literal(self.memory.alloc_stack(self.size_of(&ty))?).untainted()
                }
                ir::Instr::Store(store) => {
                    let new_value = self.eval(&registers, &store.new_value);
                    let dest = self.eval(&registers, &store.destination).as_u64().unwrap();

                    self.memory.write(dest, new_value, self.ir_module)?;
                    ValueKind::Undefined.untainted()
                }
                ir::Instr::Load { pointer, pointee } => {
                    let address = self.eval(&registers, &pointer).as_u64().unwrap();
                    self.memory.read(address, pointee)?
                }
                ir::Instr::Malloc(ir_type) => {
                    let bytes = self.size_of(ir_type);
                    ValueKind::Literal(self.memory.alloc_heap(bytes)).untainted()
                }
                ir::Instr::MallocArray(ir_type, count) => {
                    let count = self.eval(&registers, count);
                    let count = count.as_u64().unwrap();
                    let bytes = self.size_of(ir_type);
                    ValueKind::Literal(self.memory.alloc_heap(bytes * count)).untainted()
                }
                ir::Instr::Free(value) => {
                    let value = self.eval(&registers, value).kind.unwrap_literal();
                    self.memory.free_heap(value);
                    ValueKind::Literal(ir::Literal::Void).untainted()
                }
                ir::Instr::SizeOf(ty, mode) => match mode {
                    Some(SizeOfMode::Target) => {
                        // TODO: We don't support getting the target sizeof here yet...
                        todo!("sizeof<\"target\", T> is not supported yet");
                    }
                    Some(SizeOfMode::Compilation) => {
                        // If explicitly marked as compilation sizeof, then don't consider tainted
                        ValueKind::Literal(ir::Literal::new_u64(self.size_of(ty).bytes()))
                            .untainted()
                    }
                    None => {
                        // To help prevent accidentally mixing "compilation sizeof" and "target sizeof"
                        // when running code at compile time, mark the ambiguous sizeof as tainted,
                        // which we will throw an error for if it any derived value obviously leaks
                        // into the parent time.
                        ValueKind::Literal(ir::Literal::new_u64(self.size_of(ty).bytes()))
                            .tainted(Tainted::ByCompilationHostSizeof)
                    }
                },
                ir::Instr::Parameter(index) => args[usize::try_from(*index).unwrap()].clone(),
                ir::Instr::GlobalVariable(global_ref) => {
                    ValueKind::Literal(self.global_addresses.get(global_ref).unwrap().clone())
                        .untainted()
                }
                ir::Instr::BinOp(operands, BinOp::FloatOrInteger(BinOpFloatOrInteger::Add, _)) => {
                    self.add(operands, &registers)
                }
                ir::Instr::BinOp(
                    operands,
                    BinOp::FloatOrInteger(BinOpFloatOrInteger::Subtract, _),
                ) => self.sub(operands, &registers),
                ir::Instr::BinOp(
                    operands,
                    BinOp::FloatOrInteger(BinOpFloatOrInteger::Multiply, _),
                ) => self.mul(operands, &registers),
                ir::Instr::BinOp(
                    operands,
                    BinOp::FloatOrInteger(BinOpFloatOrInteger::Equals, _),
                ) => self.eq(operands, &registers),
                ir::Instr::BinOp(
                    operands,
                    BinOp::FloatOrInteger(BinOpFloatOrInteger::NotEquals, _),
                ) => self.neq(operands, &registers),
                ir::Instr::BinOp(operands, _) => {
                    todo!("Interpreter / ir::Instruction::BinOp")
                }
                /*
                ir::Instr::Checked(_, _) => todo!(),
                ir::Instr::Divide(ops, _f_or_sign) => self.div(ops, &registers)?,
                ir::Instr::Modulus(ops, _f_or_sign) => self.rem(ops, &registers)?,
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
                */
                ir::Instr::Bitcast(_, _) => todo!("Interpreter / ir::Instruction::BitCast"),
                ir::Instr::Extend(value, sign, ty) => {
                    let size = self.size_of(ty);
                    let value = self.eval(&registers, value);

                    let integer_bits = IntegerBits::new(size.into())
                        .expect("integer size is representable in interpreter");

                    let raw_data = value
                        .kind
                        .unwrap_literal()
                        .unwrap_integer()
                        .value()
                        .raw_data();

                    Value::new(
                        ValueKind::Literal(ir::Literal::Integer(
                            IntegerImmediate::new(
                                IntegerConstant::from_raw_data(raw_data, *sign),
                                integer_bits,
                            )
                            .unwrap(),
                        )),
                        value.tainted,
                    )
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
                        .fold(0, |acc, f| acc + self.size_of(&f.ir_type).bytes());

                    let subject_pointer = self.eval(&registers, subject_pointer).as_u64().unwrap();
                    ValueKind::Literal(ir::Literal::new_u64(subject_pointer.wrapping_add(offset)))
                        .untainted()
                }
                ir::Instr::ArrayAccess { .. } => {
                    todo!("Interpreter / ir::Instruction::ArrayAccess")
                }
                ir::Instr::StructLiteral(ty, values) => {
                    let mut field_values = Vec::with_capacity(values.len());

                    for value in *values {
                        field_values.push(self.eval(&registers, value));
                    }

                    let tainted = field_values
                        .iter()
                        .flat_map(|field_value| field_value.tainted)
                        .next();

                    Value {
                        kind: ValueKind::StructLiteral(StructLiteral {
                            values: field_values,
                            fields: ty.struct_fields(self.ir_module).unwrap(),
                        }),
                        tainted,
                    }
                }
                ir::Instr::IsZero(_value, _) => {
                    todo!("Interpreter / ir::Instruction::IsZero")
                }
                ir::Instr::IsNonZero(value, _) => {
                    let value = self.eval(&registers, value);

                    match &value.kind {
                        ValueKind::Undefined => ValueKind::Undefined,
                        ValueKind::Literal(literal) => {
                            ValueKind::Literal(ir::Literal::Boolean(match literal {
                                ir::Literal::Void => false,
                                ir::Literal::Boolean(x) => *x,
                                ir::Literal::Integer(intermediate) => {
                                    !intermediate.value().is_zero()
                                }
                                ir::Literal::Float32(x) => *x != 0.0,
                                ir::Literal::Float64(x) => *x != 0.0,
                                ir::Literal::NullTerminatedString(_) => true,
                                ir::Literal::Zeroed(_) => false,
                            }))
                        }
                        ValueKind::StructLiteral(_) => ValueKind::Undefined,
                    }
                    .untainted()
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
                    ValueKind::Undefined.untainted()
                }
                ir::Instr::ConditionalBreak(value, break_info) => {
                    let value = self.eval(&registers, value);

                    let should = match &value.kind {
                        ValueKind::Literal(ir::Literal::Boolean(value)) => *value,
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

                    ValueKind::Undefined.untainted()
                }
                ir::Instr::Phi(phi) => {
                    let mut found = None;

                    for incoming in phi.incoming.iter() {
                        if incoming.basicblock_id == came_from_block {
                            found = Some(self.eval(&registers, &incoming.value));
                            break;
                        }
                    }

                    found.unwrap_or(ValueKind::Undefined.untainted())
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
            .unwrap_or(ValueKind::Literal(ir::Literal::Void).untainted()))
    }

    pub fn eval(&self, registers: &Registers<'env>, value: &ir::Value<'env>) -> Value<'env> {
        match value {
            ir::Value::Literal(literal) => ValueKind::Literal(*literal).untainted(),
            ir::Value::Reference(reference) => registers.get(reference).clone(),
        }
    }

    pub fn size_of(&self, ir_type: &ir::Type<'env>) -> ByteUnits {
        size_of(ir_type, self.ir_module)
    }
}
