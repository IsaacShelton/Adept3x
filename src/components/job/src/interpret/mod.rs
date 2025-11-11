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
    ir::{self, BinOp, BinOpFloatOrInteger, BinOpFloatOrSign, BinOpSimple, IntegerImmediate},
};
use ast::SizeOfMode;
use data_units::ByteUnits;
use derivative::Derivative;
pub use error::InterpreterError;
use primitives::{IntegerBits, IntegerConstant};
use std::collections::HashMap;
pub use value::Value;
use value::{Tainted, ValueKind};

#[derive(Derivative)]
#[derivative(Debug)]
pub struct Interpreter<'env, S: SyscallHandler> {
    pub syscall_handler: S,
    steps_left: Option<u64>,
    max_steps: Option<u64>,
    ir_module: &'env ir::Ir<'env>,
    memory: Memory,
    global_addresses: HashMap<ir::GlobalRef<'env>, ir::Literal<'env>>,
    exit_value: Option<u64>,
    max_recursion_depth: usize,

    #[derivative(Debug = "ignore")]
    call_stack: Vec<CallFrame<'env>>,
}

pub struct CallFrame<'env> {
    function: &'env ir::Func<'env>,
    args: Vec<Value<'env>>,
    registers: Registers<'env>,
    ip: InstructionPointer,
    caller_fp: usize,
    came_from_block: usize,
}

impl<'env> CallFrame<'env> {
    pub fn new(
        func_ref: ir::FuncRef<'env>,
        args: Vec<Value<'env>>,
        ir_module: &'env ir::Ir<'env>,
        memory: &Memory,
    ) -> Result<Self, InterpreterError> {
        let function = &ir_module.funcs[func_ref];

        if function.ownership.is_reference() {
            return Err(InterpreterError::CannotCallForeignFunction(
                function.mangled_name.to_string(),
            ));
        }

        // TODO: Add support for calling variadic functions
        if function.is_cstyle_variadic {
            return Err(InterpreterError::CannotCallVariadicFunction);
        }

        assert_eq!(function.params.len(), args.len());

        Ok(Self {
            function,
            args,
            registers: Registers::<'env>::new(
                &function
                    .basicblocks
                    .get()
                    .expect("callee to have body lowered")[..],
            ),
            ip: InstructionPointer::default(),
            caller_fp: memory.stack_save(),
            came_from_block: 0,
        })
    }
}

impl<'env, S: SyscallHandler> Interpreter<'env, S> {
    pub fn new(
        syscall_handler: S,
        ir_module: &'env ir::Ir<'env>,
        max_steps_left: Option<u64>,
    ) -> Self {
        let mut memory = Memory::new();

        let global_addresses = ir_module
            .globals
            .iter()
            .map(|(global_ref, global)| {
                (
                    global_ref,
                    memory.alloc_permanent(size_of(&global.ir_type, ir_module)),
                )
            })
            .collect::<HashMap<_, _>>();

        Self {
            steps_left: max_steps_left,
            max_steps: max_steps_left,
            ir_module,
            memory,
            global_addresses,
            syscall_handler,
            exit_value: None,
            call_stack: Vec::with_capacity(4),
            max_recursion_depth: 512,
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
        //self.call(interpreter_entry_point, vec![])

        self.call_stack.push(CallFrame::new(
            interpreter_entry_point,
            vec![],
            self.ir_module,
            &self.memory,
        )?);

        loop {
            if let Some(returned_from_top_frame) = self.step()? {
                return Ok(returned_from_top_frame);
            }
        }
    }

    pub fn exit_value(&self) -> Option<u64> {
        self.exit_value
    }

    #[inline(always)]
    pub fn step(&mut self) -> Result<Option<Value<'env>>, InterpreterError> {
        let frame = self.call_stack.last().expect("function to run");
        let registers = &frame.registers;

        if let Some(steps_left) = &mut self.steps_left {
            if *steps_left == 0 {
                return Err(InterpreterError::TimedOut(self.max_steps.unwrap()));
            }

            *steps_left -= 1;
        }

        let instruction = &frame.function.basicblocks.get().unwrap()[frame.ip.basicblock_id]
            .instructions[frame.ip.instruction_id];
        let mut new_ip = None;

        let instr_result = match instruction {
            ir::Instr::ExitInterpreter(argument) => {
                self.exit_value = frame.registers.eval(argument).as_u64();
                return Ok(Some(ValueKind::Literal(ir::Literal::Void).untainted()));
            }
            ir::Instr::Return(value) => {
                let returned_value = value
                    .as_ref()
                    .map(|value| registers.eval(value))
                    .unwrap_or_else(|| ValueKind::Literal(ir::Literal::Void).untainted());

                self.memory.stack_restore(frame.caller_fp);
                self.call_stack.pop();

                if let Some(parent_frame) = self.call_stack.last_mut() {
                    parent_frame.registers.set(&parent_frame.ip, returned_value);
                    parent_frame.ip = parent_frame.ip.increment();
                    return Ok(None);
                } else {
                    return Ok(Some(returned_value));
                }
            }
            ir::Instr::Call(call) => {
                let mut args = Vec::with_capacity(call.args.len());

                for argument in call.args.iter() {
                    args.push(frame.registers.eval(argument));
                }

                if self.call_stack.len() >= self.max_recursion_depth {
                    return Err(InterpreterError::MaxRecursionDepthExceeded(
                        self.max_recursion_depth,
                    ));
                }

                self.call_stack.push(CallFrame::new(
                    call.func,
                    args,
                    self.ir_module,
                    &self.memory,
                )?);

                // The actual return value will be filled in when we return
                return Ok(None);
            }
            ir::Instr::Alloca(ty) => {
                ValueKind::Literal(self.memory.alloc_stack(self.size_of(&ty))?).untainted()
            }
            ir::Instr::Store(store) => {
                let new_value = registers.eval(&store.new_value);
                let dest = registers.eval(&store.destination).as_u64().unwrap();

                self.memory.write(dest, new_value, self.ir_module)?;
                ValueKind::Undefined.untainted()
            }
            ir::Instr::Load { pointer, pointee } => {
                let address = registers.eval(&pointer).as_u64().unwrap();
                self.memory.read(address, pointee)?
            }
            ir::Instr::Malloc(ir_type) => {
                let bytes = self.size_of(ir_type);
                ValueKind::Literal(self.memory.alloc_heap(bytes)).untainted()
            }
            ir::Instr::MallocArray(ir_type, count) => {
                let count = registers.eval(count);
                let count = count.as_u64().unwrap();
                let bytes = self.size_of(ir_type);
                ValueKind::Literal(self.memory.alloc_heap(bytes * count)).untainted()
            }
            ir::Instr::Free(value) => {
                let value = registers.eval(value).kind.unwrap_literal();
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
                    ValueKind::Literal(ir::Literal::new_u64(self.size_of(ty).bytes())).untainted()
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
            ir::Instr::Parameter(index) => frame.args[usize::try_from(*index).unwrap()].clone(),
            ir::Instr::GlobalVariable(global_ref) => {
                ValueKind::Literal(self.global_addresses.get(global_ref).unwrap().clone())
                    .untainted()
            }
            ir::Instr::BinOp(operands, BinOp::FloatOrInteger(BinOpFloatOrInteger::Add, _)) => {
                self.add(operands, &registers)
            }
            ir::Instr::BinOp(operands, BinOp::FloatOrInteger(BinOpFloatOrInteger::Subtract, _)) => {
                self.sub(operands, &registers)
            }
            ir::Instr::BinOp(operands, BinOp::FloatOrInteger(BinOpFloatOrInteger::Multiply, _)) => {
                self.mul(operands, &registers)
            }
            ir::Instr::BinOp(operands, BinOp::FloatOrInteger(BinOpFloatOrInteger::Equals, _)) => {
                self.eq(operands, &registers)
            }
            ir::Instr::BinOp(
                operands,
                BinOp::FloatOrInteger(BinOpFloatOrInteger::NotEquals, _),
            ) => self.neq(operands, &registers),
            ir::Instr::BinOp(operands, BinOp::FloatOrSign(BinOpFloatOrSign::Divide, _)) => {
                self.div(operands, &registers)?
            }
            ir::Instr::BinOp(operands, BinOp::FloatOrSign(BinOpFloatOrSign::Modulus, _)) => {
                self.rem(operands, &registers)?
            }
            ir::Instr::BinOp(operands, BinOp::FloatOrSign(BinOpFloatOrSign::LessThan, _)) => {
                self.lt(operands, &registers)
            }
            ir::Instr::BinOp(operands, BinOp::FloatOrSign(BinOpFloatOrSign::LessThanEq, _)) => {
                self.lte(operands, &registers)
            }
            ir::Instr::BinOp(operands, BinOp::FloatOrSign(BinOpFloatOrSign::GreaterThan, _)) => {
                self.gt(operands, &registers)
            }
            ir::Instr::BinOp(operands, BinOp::FloatOrSign(BinOpFloatOrSign::GreaterThanEq, _)) => {
                self.gte(operands, &registers)
            }
            ir::Instr::BinOp(operands, BinOp::Checked(operation)) => match operation.operator {
                ir::OverflowOperator::Add => self.checked_add(operands, &registers, operation)?,
                ir::OverflowOperator::Subtract => {
                    self.checked_sub(operands, &registers, operation)?
                }
                ir::OverflowOperator::Multiply => {
                    self.checked_mul(operands, &registers, operation)?
                }
            },
            ir::Instr::BinOp(
                operands,
                BinOp::Simple(BinOpSimple::And | BinOpSimple::BitwiseAnd),
            ) => self.bitwise_and(operands, &registers),
            ir::Instr::BinOp(operands, BinOp::Simple(BinOpSimple::Or | BinOpSimple::BitwiseOr)) => {
                self.bitwise_or(operands, &registers)
            }
            ir::Instr::BinOp(operands, BinOp::Simple(BinOpSimple::BitwiseXor)) => {
                self.bitwise_xor(operands, &registers)
            }
            ir::Instr::BinOp(operands, BinOp::Simple(BinOpSimple::LeftShift)) => {
                todo!("Interpreter / Left Shift")
            }
            ir::Instr::BinOp(operands, BinOp::Simple(BinOpSimple::LogicalRightShift)) => {
                todo!("Interpreter / Logical Right Shift Shift")
            }
            ir::Instr::BinOp(operands, BinOp::Simple(BinOpSimple::ArithmeticRightShift)) => {
                todo!("Interpreter / Arithmetic Right Shift Shift")
            }
            ir::Instr::Bitcast(_, _) => todo!("Interpreter / ir::Instr::BitCast"),
            ir::Instr::Extend(value, sign, ty) => {
                let size = self.size_of(ty);
                let value = registers.eval(value);

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
                todo!("Interpreter / ir::Instr::FloatExtend")
            }
            ir::Instr::Truncate(_, _) => todo!("Interpreter / ir::Instr::Truncate"),
            ir::Instr::TruncateFloat(_, _) => {
                todo!("Interpreter / ir::Instr::TruncateFloat")
            }
            ir::Instr::IntegerToPointer(..) => {
                todo!("Interpreter / ir::Instr::IntegerToPointer");
            }
            ir::Instr::PointerToInteger(..) => {
                todo!("Interpreter / ir::Instr::PointerToInteger");
            }

            ir::Instr::FloatToInteger(..) => {
                todo!("Interpreter / ir::Instr::FloatToInteger");
            }
            ir::Instr::IntegerToFloat(..) => {
                todo!("Interpreter / ir::Instr::IntegerToFloat");
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

                let subject_pointer = registers.eval(subject_pointer).as_u64().unwrap();
                ValueKind::Literal(ir::Literal::new_u64(subject_pointer.wrapping_add(offset)))
                    .untainted()
            }
            ir::Instr::ArrayAccess { .. } => {
                todo!("Interpreter / ir::Instr::ArrayAccess")
            }
            ir::Instr::StructLiteral(ty, values) => {
                let mut field_values = Vec::with_capacity(values.len());

                for value in *values {
                    field_values.push(registers.eval(value));
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
                todo!("Interpreter / ir::Instr::IsZero")
            }
            ir::Instr::IsNonZero(value, _) => {
                let value = registers.eval(value);

                match &value.kind {
                    ValueKind::Undefined => ValueKind::Undefined,
                    ValueKind::Literal(literal) => {
                        ValueKind::Literal(ir::Literal::Boolean(match literal {
                            ir::Literal::Void => false,
                            ir::Literal::Boolean(x) => *x,
                            ir::Literal::Integer(intermediate) => !intermediate.value().is_zero(),
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
            ir::Instr::Negate(..) => todo!("Interpreter / ir::Instr::Negate"),
            ir::Instr::BitComplement(_) => {
                todo!("Interpreter / ir::Instr::BitComplement")
            }
            ir::Instr::Break(break_info) => {
                new_ip = Some(InstructionPointer {
                    basicblock_id: break_info.basicblock_id,
                    instruction_id: 0,
                });
                ValueKind::Undefined.untainted()
            }
            ir::Instr::ConditionalBreak(value, break_info) => {
                let value = registers.eval(value);

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
                    if incoming.basicblock_id == frame.came_from_block {
                        found = Some(registers.eval(&incoming.value));
                        break;
                    }
                }

                found.unwrap_or(ValueKind::Undefined.untainted())
            }
            ir::Instr::InterpreterSyscall(syscall, supplied_args) => {
                let mut args = Vec::with_capacity(supplied_args.len());

                for supplied_arg in supplied_args.iter() {
                    args.push(registers.eval(supplied_arg));
                }

                self.syscall_handler
                    .syscall(&mut self.memory, *syscall, args)
            }
        };

        let frame = self.call_stack.last_mut().expect("function to run");
        let registers = &mut frame.registers;
        registers.set(&frame.ip, instr_result);

        if new_ip.is_some() {
            frame.came_from_block = frame.ip.basicblock_id;
        }

        frame.ip = new_ip.unwrap_or_else(|| frame.ip.increment());
        Ok(None)
    }

    pub fn size_of(&self, ir_type: &ir::Type<'env>) -> ByteUnits {
        size_of(ir_type, self.ir_module)
    }
}
