use crate::{
    Continuation, EndInstrKind, Executable, ExecutionCtx, Executor, InstrKind, InstrRef,
    conform::UnaryImplicitCast,
    ir::{self, Literal},
    module_graph::ModuleView,
    repr::{self, Compiler, FuncBody, FuncHead, TypeKind},
    target_layout::TargetLayout,
};
use arena::Id;
use by_address::ByAddress;
use core::f64;
use data_units::BitUnits;
use derivative::Derivative;
use diagnostics::ErrorDiagnostic;
use num_bigint::BigInt;
use num_traits::{ConstZero, ToPrimitive};
use primitives::{FloatSize, IntegerBits, IntegerSign};
use source_files::Source;
use target::Target;

#[derive(Clone, Derivative)]
#[derivative(Debug, PartialEq, Eq, Hash)]
pub struct LowerFunctionBody<'env> {
    view: ModuleView<'env>,

    #[derivative(Debug = "ignore")]
    compiler: ByAddress<&'env Compiler<'env>>,

    func: ir::FuncRef<'env>,
    head: ByAddress<&'env FuncHead<'env>>,
    body: ByAddress<&'env FuncBody<'env>>,

    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    current_basicblock: usize,

    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    current_node_index: usize,

    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    basicblocks: Vec<Vec<ir::Instr<'env>>>,
}

impl<'env> LowerFunctionBody<'env> {
    pub fn new(
        view: ModuleView<'env>,
        compiler: &'env Compiler<'env>,
        func: ir::FuncRef<'env>,
        head: &'env FuncHead<'env>,
        body: &'env FuncBody<'env>,
    ) -> Self {
        Self {
            view,
            compiler: ByAddress(compiler),
            func,
            head: ByAddress(head),
            body: ByAddress(body),
            current_basicblock: 0,
            current_node_index: 0,
            basicblocks: vec![vec![]],
        }
    }
}

impl<'env> Executable<'env> for LowerFunctionBody<'env> {
    type Output = ();

    fn execute(
        self,
        _executor: &Executor<'env>,
        ctx: &mut ExecutionCtx<'env>,
    ) -> Result<Self::Output, Continuation<'env>> {
        let ir = self.view.web.graph(self.view.graph, |graph| graph.ir);
        let func = &ir.funcs[self.func];

        // This may be easier if the CFG representation is already in basicblocks...
        // Otherwise, we have to convert it to basicblocks here anyway...

        // We can then also make variable lookup way faster by using hashmaps...
        // Possibly either by using a "time" score of which is declared at each time
        // within a basicblock, or just by having basicblocks be processed in reverse
        // post order, which I think we already do, and from top to bottom.

        // That would also greatly speed up the time taken to compute the
        // immediate dominators tree too I think.

        // We would probably want to keep all control-flow modifying constructs
        // within the resolution stage anyway, so they can correctly impact
        // the control-flow sensitive type checking.

        // TODO: Here is where we will do monomorphization (but only for the function body)...

        let mut builder = IrBuilder::new(&self.body);

        for bb_id in self.body.post_order.iter().copied() {
            let bb_index = bb_id.into_usize();
            let bb = &self.body.cfg.get_unsafe(bb_id);
            builder.set_position(bb_index);

            for instr in bb.instrs.iter() {
                let result = match &instr.kind {
                    InstrKind::Phi(items, conform_behavior) => todo!("lower phi"),
                    InstrKind::Name(_) => todo!(),
                    InstrKind::Parameter(_, _, index) => builder.push(ir::Instr::Parameter(*index)),
                    InstrKind::Declare(_, _, instr_ref, unary_implicit_cast) => todo!(),
                    InstrKind::Assign(left, right) => builder.push(ir::Instr::Store(ir::Store {
                        new_value: builder.get_output(*right),
                        destination: builder.get_output(*left),
                    })),
                    InstrKind::BinOp(instr_ref, basic_binary_operator, instr_ref1, language) => {
                        todo!()
                    }
                    InstrKind::BooleanLiteral(value) => ir::Literal::Boolean(*value).into(),
                    InstrKind::IntegerLiteral(integer) => {
                        let value = integer.value();
                        let cfg_ty = instr.typed.unwrap().0;

                        let result_value = match &cfg_ty.kind {
                            TypeKind::IntegerLiteral(_) => ir::Literal::Void.into(),
                            TypeKind::IntegerLiteralInRange(min, max) => {
                                let (bits, sign) =
                                    bits_and_sign_for_invisible_integer_in_range(min, max)
                                        .map_err(|_| {
                                            ErrorDiagnostic::new(
                                                "Integer range too large to represent",
                                                instr.source,
                                            )
                                        })?;
                                value_for_bit_integer(value, bits, sign, instr.source)?
                            }
                            TypeKind::BitInteger(bits, sign) => {
                                value_for_bit_integer(value, *bits, *sign, instr.source)?
                            }
                            _ => unreachable!(),
                        };

                        result_value
                    }
                    InstrKind::FloatLiteral(_) => todo!(),
                    InstrKind::AsciiCharLiteral(_) => todo!(),
                    InstrKind::Utf8CharLiteral(_) => todo!(),
                    InstrKind::StringLiteral(_) => todo!(),
                    InstrKind::NullTerminatedStringLiteral(cstr) => {
                        ir::Literal::NullTerminatedString(cstr).into()
                    }
                    InstrKind::NullLiteral => todo!(),
                    InstrKind::VoidLiteral => todo!(),
                    InstrKind::Call(call_instr) => todo!(),
                    InstrKind::DeclareAssign(_, instr_ref, unary_implicit_cast) => todo!(),
                    InstrKind::Member(instr_ref, _, privacy) => todo!(),
                    InstrKind::ArrayAccess(instr_ref, instr_ref1) => todo!(),
                    InstrKind::StructLiteral(struct_literal_instr) => todo!(),
                    InstrKind::UnaryOperation(unary_operator, instr_ref) => todo!(),
                    InstrKind::SizeOf(_, size_of_mode) => todo!(),
                    InstrKind::SizeOfValue(instr_ref, size_of_mode) => todo!(),
                    InstrKind::InterpreterSyscall(interpreter_syscall_instr) => todo!(),
                    InstrKind::IntegerPromote(instr_ref) => todo!(),
                    InstrKind::ConformToBool(instr_ref, language, unary_implicit_cast) => todo!(),
                    InstrKind::Is(instr_ref, _) => todo!(),
                    InstrKind::LabelLiteral(_) => todo!(),
                };

                builder.push_output(result);
            }

            let instr_end = &bb.end;

            let result = match &instr_end.kind {
                EndInstrKind::IncompleteGoto(_) => todo!(),
                EndInstrKind::IncompleteBreak => todo!(),
                EndInstrKind::IncompleteContinue => todo!(),
                EndInstrKind::Return(value, unary_implicit_cast) => {
                    let value: Option<ir::Value<'env>> = value
                        .map(|value| {
                            let cfg_ty = self.head.return_type.0;
                            let value = builder.get_output(value);

                            to_ir_type(ctx, self.view.target(), cfg_ty).map(|ir_ty| {
                                perform_unary_implicit_cast(
                                    value,
                                    &ir_ty,
                                    unary_implicit_cast.as_ref(),
                                )
                            })
                        })
                        .transpose()?;

                    builder.push(ir::Instr::Return(value));
                    ir::Literal::Void.into()
                }
                EndInstrKind::Jump(basic_block_id, value, unaliased_type) => {
                    let value = value.map(|value| builder.get_output(value));

                    // TODO: Perform conform before assigning value
                    eprintln!("warning: casts for jumps are not performed yet");

                    builder.push(ir::Instr::Break(ir::Break {
                        basicblock_id: basic_block_id.into_usize(),
                    }));

                    value.unwrap_or(ir::Literal::Void.into())
                }
                EndInstrKind::Branch(
                    instr_ref,
                    basic_block_id,
                    basic_block_id1,
                    break_continue,
                ) => todo!(),
                EndInstrKind::NewScope(basic_block_id, basic_block_id1) => todo!(),
                EndInstrKind::Unreachable => todo!(),
            };

            builder.push_output(result);
        }

        let basicblocks =
            &*ctx.alloc_slice_fill_iter(builder.basicblocks.into_iter().map(|instrs| {
                ir::BasicBlock {
                    instructions: &*ctx.alloc_slice_fill_iter(instrs.into_iter()),
                }
            }));

        let ir_func = &ir.funcs[self.func];
        ir_func.basicblocks.set(basicblocks).unwrap();
        Ok(())
    }
}

#[derive(Debug)]
struct IrBuilder<'env> {
    basicblocks: Vec<Vec<ir::Instr<'env>>>,
    outputs: Vec<Vec<Option<ir::Value<'env>>>>,
    current_bb_index: Option<usize>,
    current_cfg_instr_index: usize,
}

impl<'env> IrBuilder<'env> {
    pub fn new(body: &FuncBody<'env>) -> Self {
        let outputs = Vec::from_iter(
            body.cfg
                .basicblocks
                .values()
                .map(|bb| Vec::from_iter(std::iter::repeat_n(None, bb.instrs.len() + 1))),
        );

        let basicblocks = Vec::from_iter(body.cfg.basicblocks.values().map(|_| Vec::new()));

        Self {
            basicblocks,
            outputs,
            current_bb_index: None,
            current_cfg_instr_index: 0,
        }
    }

    pub fn set_position(&mut self, new_bb_index: usize) {
        self.current_bb_index = Some(new_bb_index);
        self.current_cfg_instr_index = 0;
    }

    pub fn push(&mut self, instr: ir::Instr<'env>) -> ir::Value<'env> {
        let current_bb_index = self.current_bb_index.unwrap();
        let current_block = &mut self.basicblocks[current_bb_index];
        current_block.push(instr);

        ir::Value::Reference(ir::ValueReference {
            basicblock_id: current_bb_index,
            instruction_id: current_block.len() - 1,
        })
    }

    pub fn push_output(&mut self, value: ir::Value<'env>) {
        self.outputs[self.current_bb_index.unwrap()][self.current_cfg_instr_index] = Some(value);
        self.current_cfg_instr_index += 1;
    }

    pub fn get_output(&self, instr_ref: InstrRef) -> ir::Value<'env> {
        *self.outputs[instr_ref.basicblock.into_usize()][instr_ref.instr_or_end as usize]
            .as_ref()
            .unwrap()
    }
}

fn value_for_bit_integer(
    value: &BigInt,
    bits: IntegerBits,
    sign: IntegerSign,
    source: Source,
) -> Result<ir::Value, ErrorDiagnostic> {
    match (bits, sign) {
        (IntegerBits::Bits8, IntegerSign::Signed) => {
            value.try_into().map(Literal::Signed8).map_err(|_| "i8")
        }
        (IntegerBits::Bits8, IntegerSign::Unsigned) => {
            value.try_into().map(Literal::Unsigned8).map_err(|_| "u8")
        }
        (IntegerBits::Bits16, IntegerSign::Signed) => {
            value.try_into().map(Literal::Signed16).map_err(|_| "i16")
        }
        (IntegerBits::Bits16, IntegerSign::Unsigned) => {
            value.try_into().map(Literal::Unsigned16).map_err(|_| "u16")
        }
        (IntegerBits::Bits32, IntegerSign::Signed) => {
            value.try_into().map(Literal::Signed32).map_err(|_| "i32")
        }
        (IntegerBits::Bits32, IntegerSign::Unsigned) => {
            value.try_into().map(Literal::Unsigned32).map_err(|_| "u32")
        }
        (IntegerBits::Bits64, IntegerSign::Signed) => {
            value.try_into().map(Literal::Signed64).map_err(|_| "i64")
        }
        (IntegerBits::Bits64, IntegerSign::Unsigned) => {
            value.try_into().map(Literal::Unsigned64).map_err(|_| "u64")
        }
    }
    .map(|literal| ir::Value::Literal(literal))
    .map_err(|expected_type| {
        ErrorDiagnostic::new(
            format!("Cannot fit value {} in '{}'", value, expected_type),
            source,
        )
    })
}

fn bits_and_sign_for_invisible_integer(value: &BigInt) -> Result<(IntegerBits, IntegerSign), ()> {
    bits_and_sign_for_invisible_integer_in_range(value, value)
}

fn bits_and_sign_for_invisible_integer_in_range(
    min: &BigInt,
    max: &BigInt,
) -> Result<(IntegerBits, IntegerSign), ()> {
    let signed = *min < BigInt::ZERO || *max < BigInt::ZERO;
    let bits = IntegerBits::new(BitUnits::of(min.bits().max(max.bits()) + signed as u64));
    bits.map(|bits| (bits, IntegerSign::new(signed))).ok_or(())
}

fn perform_unary_implicit_cast<'env>(
    value: ir::Value<'env>,
    ty: &ir::Type<'env>,
    cast: Option<&UnaryImplicitCast>,
) -> ir::Value<'env> {
    let Some(cast) = cast else {
        return value;
    };

    use IntegerBits::*;
    use IntegerSign::*;

    match *cast {
        UnaryImplicitCast::SpecializeBoolean(_) => todo!(),
        UnaryImplicitCast::SpecializeInteger(value) => match ty {
            ir::Type::Bool => ir::Literal::Boolean(*value != BigInt::ZERO).into(),
            ir::Type::I(Bits8, Signed) => ir::Literal::Signed8(value.try_into().unwrap()).into(),
            ir::Type::I(Bits16, Signed) => ir::Literal::Signed16(value.try_into().unwrap()).into(),
            ir::Type::I(Bits32, Signed) => ir::Literal::Signed32(value.try_into().unwrap()).into(),
            ir::Type::I(Bits64, Signed) => ir::Literal::Signed64(value.try_into().unwrap()).into(),
            ir::Type::I(Bits8, Unsigned) => {
                ir::Literal::Unsigned8(value.try_into().unwrap()).into()
            }
            ir::Type::I(Bits16, Unsigned) => {
                ir::Literal::Unsigned16(value.try_into().unwrap()).into()
            }
            ir::Type::I(Bits32, Unsigned) => {
                ir::Literal::Unsigned32(value.try_into().unwrap()).into()
            }
            ir::Type::I(Bits64, Unsigned) => {
                ir::Literal::Unsigned64(value.try_into().unwrap()).into()
            }
            ir::Type::F(FloatSize::Bits32) => {
                ir::Literal::Float32(value.to_f32().unwrap_or_else(|| {
                    if *value < BigInt::ZERO {
                        f32::NEG_INFINITY
                    } else {
                        f32::INFINITY
                    }
                }))
                .into()
            }
            ir::Type::F(FloatSize::Bits64) => {
                ir::Literal::Float64(value.to_f64().unwrap_or_else(|| {
                    if *value < BigInt::ZERO {
                        f64::NEG_INFINITY
                    } else {
                        f64::INFINITY
                    }
                }))
                .into()
            }
            _ => panic!("Cannot specialize integer for type {:?}", ty),
        },
        UnaryImplicitCast::SpecializeFloat(not_nan) => todo!("specialize float"),
        UnaryImplicitCast::SpecializePointerOuter(unaliased_type) => {
            todo!("specialize pointer outer")
        }
        UnaryImplicitCast::SpecializeAsciiChar(_) => todo!("specialize ascii char"),
    }
}

fn to_ir_type<'env>(
    ctx: &mut ExecutionCtx<'env>,
    target: &Target,
    ty: &repr::Type,
) -> Result<ir::Type<'env>, ErrorDiagnostic> {
    Ok(match &ty.kind {
        TypeKind::IntegerLiteral(_) => ir::Type::Void,
        TypeKind::IntegerLiteralInRange(min, max) => {
            let (bits, sign) =
                bits_and_sign_for_invisible_integer_in_range(min, max).map_err(|_| {
                    ErrorDiagnostic::new("Integer range too large to represent", ty.source)
                })?;

            ir::Type::I(bits, sign)
        }
        TypeKind::FloatLiteral(_) => ir::Type::Void,
        TypeKind::BooleanLiteral(_) => ir::Type::Void,
        TypeKind::NullLiteral => ir::Type::Void,
        TypeKind::AsciiCharLiteral(_) => ir::Type::Void,
        TypeKind::Boolean => ir::Type::Bool,
        TypeKind::BitInteger(bits, sign) => ir::Type::I(*bits, *sign),
        TypeKind::CInteger(c_integer, sign) => {
            let bytes = target.c_integer_bytes(*c_integer);
            let Some(bits) = IntegerBits::new(bytes.to_bits()) else {
                return Err(ErrorDiagnostic::new(
                    "C integer is larger that supported",
                    ty.source,
                ));
            };

            ir::Type::I(
                bits,
                sign.unwrap_or_else(|| target.default_c_integer_sign(*c_integer)),
            )
        }
        TypeKind::SizeInteger(integer_sign) => todo!(),
        TypeKind::Floating(float_size) => ir::Type::F(*float_size),
        TypeKind::Ptr(_) => todo!(),
        TypeKind::Void => ir::Type::Void,
        TypeKind::Never => ir::Type::Void,
        TypeKind::FixedArray(_, _) => todo!(),
        TypeKind::UserDefined(user_defined_type) => todo!(),
        TypeKind::Polymorph(_) => panic!("Cannot convert unspecialized polymorph to IR type!"),
        TypeKind::DirectLabel(_) => ir::Type::Void,
    })
}
