mod deref_dest;
mod ir_builder;

use crate::{
    CfgValue, Continuation, EndInstrKind, Executable, ExecutionCtx, Executor, InstrKind,
    RevPostOrderIterWithEnds, Suspend, SuspendMany,
    conform::UnaryCast,
    execution::lower::{
        LowerFunctionHead, LowerType, bits_and_sign_for_invisible_integer_in_range,
        function_body::deref_dest::DerefDest, value_for_bit_integer,
    },
    ir,
    module_graph::ModuleView,
    repr::{Compiler, FuncBody, FuncHead, TypeKind, VariableId, VariableRef},
    sub_task::SubTask,
};
use arena::Id;
use by_address::ByAddress;
use core::f64;
use derivative::Derivative;
use diagnostics::ErrorDiagnostic;
use ir_builder::*;
use num_bigint::BigInt;
use num_traits::ToPrimitive;
use primitives::{FloatOrInteger, FloatSize, IntegerBits, IntegerSign};
use source_files::Source;
use target::Target;

#[derive(Clone, Derivative)]
#[derivative(Debug, PartialEq, Eq, Hash)]
pub struct LowerFunctionBody<'env> {
    view: &'env ModuleView<'env>,

    #[derivative(Debug = "ignore")]
    compiler: ByAddress<&'env Compiler<'env>>,

    func: ir::FuncRef<'env>,
    head: ByAddress<&'env FuncHead<'env>>,
    body: ByAddress<&'env FuncBody<'env>>,

    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    builder: Option<IrBuilder<'env>>,

    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    rev_post_order: Option<RevPostOrderIterWithEnds>,

    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    suspend_on_func: Suspend<'env, ir::FuncRef<'env>>,

    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    variables: usize,

    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    lowered_type: Suspend<'env, ir::Type<'env>>,

    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    deref_dest: Option<DerefDest<'env>>,

    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    lowered_ir_types: SuspendMany<'env, ir::Type<'env>>,
}

impl<'env> LowerFunctionBody<'env> {
    pub fn new(
        view: &'env ModuleView<'env>,
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
            builder: None,
            rev_post_order: None,
            suspend_on_func: None,
            variables: 0,
            lowered_type: None,
            deref_dest: None,
            lowered_ir_types: None,
        }
    }
}

impl<'env> Executable<'env> for LowerFunctionBody<'env> {
    type Output = ();

    fn execute(
        mut self,
        executor: &Executor<'env>,
        ctx: &mut ExecutionCtx<'env>,
    ) -> Result<Self::Output, Continuation<'env>> {
        // TODO: Before here is where we'll do monomorphization (but only for the function body)...
        // Should function body monomorphization be a separate step or combined with lowering?

        let cfg = self.body.cfg;

        let builder = self.builder.get_or_insert_with(|| {
            let mut builder = IrBuilder::new(&self.body);
            builder.set_position(0);
            builder
        });

        while self.variables < self.body.variables.len() {
            let variable = self
                .body
                .variables
                .get(unsafe { VariableRef::from_raw(VariableId::from_usize(self.variables)) });

            let Some(lowered_type) = executor.demand(self.lowered_type) else {
                return suspend!(
                    self.lowered_type,
                    executor.request(LowerType::new(self.view, &self.compiler, &variable.ty.0)),
                    ctx
                );
            };

            builder.push(ir::Instr::Alloca(lowered_type));
            self.variables += 1;
            self.lowered_type = None;
        }

        let get_variable_alloca = |variable_ref: VariableRef<'env>| {
            ir::Value::Reference(ir::ValueReference {
                basicblock_id: 0,
                instruction_id: variable_ref.into_raw().into_usize(),
            })
        };

        let rev_post_order = self
            .rev_post_order
            .get_or_insert_with(|| RevPostOrderIterWithEnds::new(self.body.post_order));

        let builtin_types = self.compiler.builtin_types;

        while let Some(instr_ref) = rev_post_order.peek() {
            let bb_id = instr_ref.basicblock;
            let bb_index = bb_id.into_usize();
            let bb = &self.body.cfg.get_unsafe(bb_id);
            builder.set_position(bb_index);

            if (instr_ref.instr_or_end as usize) < bb.instrs.len() {
                let instr = &bb.instrs[instr_ref.instr_or_end as usize];

                let result = match &instr.kind {
                    InstrKind::Phi {
                        possible_incoming,
                        conform_behavior: _,
                    } => {
                        let unified_type = instr.typed.as_ref().unwrap();

                        let Some(lowered_type) = executor.demand(self.lowered_type) else {
                            return suspend!(
                                self.lowered_type,
                                executor.request(LowerType::new(
                                    self.view,
                                    &self.compiler,
                                    unified_type.0
                                )),
                                ctx
                            );
                        };

                        let incoming =
                            possible_incoming
                                .iter()
                                .map(|(bb, _uncasted_value)| ir::PhiIncoming {
                                    basicblock_id: bb.into_usize(),
                                    value: builder
                                        .get_output(CfgValue::Instr(cfg.get_end_ref(*bb))),
                                });

                        builder.push(ir::Instr::Phi(ir::Phi {
                            ir_type: lowered_type,
                            incoming: ctx.alloc_slice_fill_iter(incoming.into_iter()),
                        }))
                    }
                    InstrKind::Name(_, variable_ref) => get_variable_alloca(variable_ref.unwrap()),
                    InstrKind::Parameter(_, _, index, _) => {
                        builder.push(ir::Instr::Parameter(*index))
                    }
                    InstrKind::Declare(_, _, value, unary_cast, variable_ref) => {
                        if let Some(value) = value {
                            let variable_ref = variable_ref.unwrap();
                            let variable_type = &self.body.variables.get(variable_ref).ty.0;

                            let Some(lowered_type) = executor.demand(self.lowered_type) else {
                                return suspend!(
                                    self.lowered_type,
                                    executor.request(LowerType::new(
                                        self.view,
                                        &self.compiler,
                                        variable_type
                                    )),
                                    ctx
                                );
                            };

                            let value = builder.get_output(*value);
                            let stack_memory = get_variable_alloca(variable_ref);

                            let value = perform_unary_cast_to(
                                ctx,
                                builder,
                                value,
                                &lowered_type,
                                unary_cast.as_ref(),
                                self.view.target(),
                                instr.source,
                            )?;

                            builder.push(ir::Instr::Store(ir::Store {
                                new_value: value,
                                destination: stack_memory,
                            }));
                        }

                        ir::Literal::Void.into()
                    }
                    InstrKind::Assign {
                        dest,
                        src,
                        src_cast: cast,
                    } => {
                        let new_value = builder.get_output(*src);

                        let TypeKind::Deref(to_ty) = cfg.get_typed(*dest, builtin_types).0.kind
                        else {
                            return Err(ErrorDiagnostic::new(
                                "Could not assign value, left hand side is not mutable",
                                instr.source,
                            )
                            .into());
                        };

                        let (dest, to_ir_ty) = execute_sub_task!(
                            self,
                            self.deref_dest.get_or_insert_with(|| DerefDest::new(
                                self.view,
                                &self.compiler,
                                builder.get_output(*dest),
                                to_ty,
                                instr.source
                            )),
                            executor,
                            ctx,
                            builder
                        );

                        let new_value = perform_unary_cast_to(
                            ctx,
                            builder,
                            new_value,
                            &to_ir_ty,
                            cast.as_ref(),
                            self.view.target(),
                            instr.source,
                        )?;

                        builder.push(ir::Instr::Store(ir::Store {
                            new_value,
                            destination: dest,
                        }))
                    }
                    InstrKind::BinOp(instr_ref, basic_binary_operator, instr_ref1, language) => {
                        todo!()
                    }
                    InstrKind::BooleanLiteral(value) => ir::Literal::Boolean(*value).into(),
                    InstrKind::IntegerLiteral(integer) => {
                        let value = integer.value();

                        let result_value = match &instr.typed.unwrap().0.kind {
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
                            _ => {
                                return Err(ErrorDiagnostic::new(
                                    "Cannot lower integer literal to unsupported type",
                                    instr.source,
                                )
                                .into());
                            }
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
                    InstrKind::VoidLiteral => ir::Value::Literal(ir::Literal::Void),
                    InstrKind::Call(call, target) => {
                        let call_target = target
                            .as_ref()
                            .expect("call without target cannot be lowered");

                        let Some(ir_func_ref) = executor.demand(self.suspend_on_func) else {
                            return suspend!(
                                self.suspend_on_func,
                                executor.request(LowerFunctionHead::new(
                                    call_target.view,
                                    &self.compiler,
                                    call_target.callee,
                                )),
                                ctx
                            );
                        };

                        let Some(param_ir_types) = executor.demand_many(&self.lowered_ir_types)
                        else {
                            return suspend_many!(
                                self.lowered_ir_types,
                                call.args
                                    .iter()
                                    .copied()
                                    .enumerate()
                                    .map(|(i, _)| {
                                        executor.request(LowerType::new(
                                            self.view,
                                            &self.compiler,
                                            &call_target.get_param_or_arg_type(i).0,
                                        ))
                                    })
                                    .collect(),
                                ctx
                            );
                        };

                        // Perform unary casts for all values to parameter types
                        let mut args = vec![];
                        for ((arg_instr, unary_cast), param_ir_ty) in call
                            .args
                            .iter()
                            .copied()
                            .zip(call_target.arg_casts.iter())
                            .zip(param_ir_types.iter())
                        {
                            let value = builder.get_output(arg_instr);

                            let value = perform_unary_cast_to(
                                ctx,
                                builder,
                                value,
                                param_ir_ty,
                                unary_cast.as_ref(),
                                self.view.target(),
                                instr.source,
                            )?;

                            args.push(value);
                        }

                        // Reset ability to suspend on IR function head
                        self.suspend_on_func = None;

                        builder.push(ir::Instr::Call(ir::Call {
                            func: ir_func_ref,
                            args: ctx.alloc_slice_fill_iter(args.into_iter()),
                            unpromoted_variadic_arg_types: ctx.alloc_slice_fill_iter(
                                param_ir_types
                                    .into_iter()
                                    .skip(call_target.callee.params.required.len()),
                            ),
                        }))
                    }
                    InstrKind::DeclareAssign(_, value, unary_cast, variable_ref) => {
                        let variable_ref = variable_ref.unwrap();
                        let value = builder.get_output(*value);

                        let Some(lowered_type) = executor.demand(self.lowered_type) else {
                            return suspend!(
                                self.lowered_type,
                                executor.request(LowerType::new(
                                    self.view,
                                    &self.compiler,
                                    &instr.typed.unwrap().0
                                )),
                                ctx
                            );
                        };

                        let value = perform_unary_cast_to(
                            ctx,
                            builder,
                            value,
                            &lowered_type,
                            unary_cast.as_ref(),
                            self.view.target(),
                            instr.source,
                        )?;

                        builder.push(ir::Instr::Store(ir::Store {
                            new_value: value,
                            destination: get_variable_alloca(variable_ref),
                        }))
                    }
                    InstrKind::Member(instr_ref, _, privacy) => todo!(),
                    InstrKind::ArrayAccess(instr_ref, instr_ref1) => todo!(),
                    InstrKind::StructLiteral(struct_literal_instr, _) => {
                        todo!("lower struct literal")
                    }
                    InstrKind::UnaryOperation(unary_operator, cfg_value, cast) => {
                        match unary_operator {
                            ast::UnaryOperator::Math(ast::UnaryMathOperator::Not) => {
                                let value = perform_unary_cast_to(
                                    ctx,
                                    builder,
                                    builder.get_output(*cfg_value),
                                    &ir::Type::Bool,
                                    cast.as_ref(),
                                    self.view.target(),
                                    instr.source,
                                )?;

                                builder.push(ir::Instr::IsZero(value, FloatOrInteger::Integer))
                            }
                            ast::UnaryOperator::Math(ast::UnaryMathOperator::Negate) => todo!(),
                            ast::UnaryOperator::Math(ast::UnaryMathOperator::IsNonZero) => todo!(),
                            ast::UnaryOperator::Math(ast::UnaryMathOperator::BitComplement) => {
                                todo!()
                            }
                            ast::UnaryOperator::AddressOf => todo!(),
                            ast::UnaryOperator::Dereference => todo!(),
                        }
                    }
                    InstrKind::SizeOf(_, size_of_mode) => todo!(),
                    InstrKind::SizeOfValue(instr_ref, size_of_mode) => todo!(),
                    InstrKind::InterpreterSyscall(interpreter_syscall_instr) => todo!(),
                    InstrKind::IntegerPromote(instr_ref) => todo!(),
                    InstrKind::ConformToBool(value, _language, unary_cast) => {
                        let Some(lowered_type) = executor.demand(self.lowered_type) else {
                            return suspend!(
                                self.lowered_type,
                                executor.request(LowerType::new(
                                    self.view,
                                    &self.compiler,
                                    &instr.typed.unwrap().0
                                )),
                                ctx
                            );
                        };

                        perform_unary_cast_to(
                            ctx,
                            builder,
                            builder.get_output(*value),
                            &lowered_type,
                            unary_cast.as_ref(),
                            self.view.target(),
                            instr.source,
                        )?
                    }
                    InstrKind::Is(instr_ref, _) => todo!(),
                    InstrKind::LabelLiteral(_) => todo!(),
                };

                self.lowered_type = None;
                self.deref_dest = None;
                self.lowered_ir_types = None;

                builder.push_output(result);
                rev_post_order.next(cfg, self.body.post_order);
                continue;
            }

            let end_instr = &bb.end;

            let result = match &end_instr.kind {
                EndInstrKind::IncompleteGoto(_) => todo!(),
                EndInstrKind::IncompleteBreak => todo!(),
                EndInstrKind::IncompleteContinue => todo!(),
                EndInstrKind::Return(return_value, unary_cast) => {
                    let return_instr = ir::Instr::Return(if let CfgValue::Void = return_value {
                        None
                    } else {
                        let Some(lowered_type) = executor.demand(self.lowered_type) else {
                            return suspend!(
                                self.lowered_type,
                                executor.request(LowerType::new(
                                    self.view,
                                    &self.compiler,
                                    self.head.return_type.0
                                )),
                                ctx
                            );
                        };

                        // TODO: Technically, this should be expanded to handle the case
                        // where we return void as a value.
                        Some(perform_unary_cast_to(
                            ctx,
                            builder,
                            builder.get_output(*return_value),
                            &lowered_type,
                            unary_cast.as_ref(),
                            self.view.target(),
                            end_instr.source,
                        )?)
                    });

                    builder.push(return_instr);
                    ir::Literal::Void.into()
                }
                EndInstrKind::Jump(basic_block_id, value, cast, to_ty) => {
                    if let Some(to_ty) = to_ty {
                        let Some(lowered_type) = executor.demand(self.lowered_type) else {
                            return suspend!(
                                self.lowered_type,
                                executor.request(LowerType::new(
                                    self.view,
                                    &self.compiler,
                                    to_ty.0
                                )),
                                ctx
                            );
                        };

                        let value = perform_unary_cast_to(
                            ctx,
                            builder,
                            builder.get_output(*value),
                            &lowered_type,
                            cast.as_ref(),
                            self.view.target(),
                            end_instr.source,
                        )?;

                        builder.push(ir::Instr::Break(ir::Break {
                            basicblock_id: basic_block_id.into_usize(),
                        }));
                        value
                    } else {
                        builder.push(ir::Instr::Break(ir::Break {
                            basicblock_id: basic_block_id.into_usize(),
                        }));
                        ir::Literal::Void.into()
                    }
                }
                EndInstrKind::Branch(condition, when_true, when_false, break_continue) => {
                    let condition = builder.get_output(*condition);

                    builder.push(ir::Instr::ConditionalBreak(
                        condition,
                        ir::ConditionalBreak {
                            true_basicblock_id: when_true.into_usize(),
                            false_basicblock_id: when_false.into_usize(),
                        },
                    ))
                }
                EndInstrKind::NewScope(in_scope, _close_scope) => {
                    builder.push(ir::Instr::Break(ir::Break {
                        basicblock_id: in_scope.into_usize(),
                    }));

                    ir::Literal::Void.into()
                }
                EndInstrKind::Unreachable => todo!(),
            };

            self.lowered_type = None;
            builder.push_output(result);
            rev_post_order.next(cfg, self.body.post_order);
        }

        // Collect lowered instructions into IR basicblocks
        let basicblocks = &*ctx.alloc_slice_fill_iter(builder.finish().into_iter().map(|instrs| {
            ir::BasicBlock {
                instructions: &*ctx.alloc_slice_fill_iter(instrs.into_iter()),
            }
        }));

        // Attach body to function
        let ir = self.view.web.graph(self.view.graph, |graph| graph.ir);
        let ir_func = &ir.funcs[self.func];

        ir_func.basicblocks.set(basicblocks).unwrap();
        Ok(())
    }
}

fn perform_unary_cast_to<'env>(
    ctx: &mut ExecutionCtx<'env>,
    builder: &mut IrBuilder<'env>,
    value: ir::Value<'env>,
    to_ty: &ir::Type<'env>,
    cast: Option<&UnaryCast<'env>>,
    target: &Target,
    source: Source,
) -> Result<ir::Value<'env>, ErrorDiagnostic> {
    let mut value = value;
    let mut next_cast = cast;

    loop {
        let Some(cast) = next_cast.take() else {
            return Ok(value);
        };

        use IntegerBits::*;
        use IntegerSign::*;

        let cast_failed = |_| {
            ErrorDiagnostic::ice(
                format!("Failed to perform cast to {:?}", to_ty),
                Some(source),
            )
        };

        value = match *cast {
            UnaryCast::SpecializeBoolean(value) => ir::Literal::Boolean(value).into(),
            UnaryCast::SpecializeInteger(value) => match to_ty {
                ir::Type::Bool => ir::Literal::Boolean(*value != BigInt::ZERO).into(),
                ir::Type::I(Bits8, Signed) => {
                    ir::Literal::Signed8(value.try_into().map_err(cast_failed)?).into()
                }
                ir::Type::I(Bits16, Signed) => {
                    ir::Literal::Signed16(value.try_into().map_err(cast_failed)?).into()
                }
                ir::Type::I(Bits32, Signed) => {
                    ir::Literal::Signed32(value.try_into().map_err(cast_failed)?).into()
                }
                ir::Type::I(Bits64, Signed) => {
                    ir::Literal::Signed64(value.try_into().map_err(cast_failed)?).into()
                }
                ir::Type::I(Bits8, Unsigned) => {
                    ir::Literal::Unsigned8(value.try_into().map_err(cast_failed)?).into()
                }
                ir::Type::I(Bits16, Unsigned) => {
                    ir::Literal::Unsigned16(value.try_into().map_err(cast_failed)?).into()
                }
                ir::Type::I(Bits32, Unsigned) => {
                    ir::Literal::Unsigned32(value.try_into().map_err(cast_failed)?).into()
                }
                ir::Type::I(Bits64, Unsigned) => {
                    ir::Literal::Unsigned64(value.try_into().map_err(cast_failed)?).into()
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
                _ => panic!("Cannot specialize integer for type {:?}", to_ty),
            },
            UnaryCast::SpecializeFloat(not_nan) => todo!("specialize float"),
            UnaryCast::SpecializePointerOuter(unaliased_type) => {
                todo!("specialize pointer outer")
            }
            UnaryCast::SpecializeAsciiChar(_) => todo!("specialize ascii char"),
            UnaryCast::Dereference { after_deref, then } => {
                let after_deref = todo!("perform_unary_cast_to UnaryCast::Deference async"); // to_ir_type(ctx, target, after_deref.0)?;
                next_cast = then;

                builder.push(ir::Instr::Load {
                    pointer: value,
                    pointee: after_deref,
                })
            }
            UnaryCast::Extend(from_sign) => {
                assert!(to_ty.is_i());
                builder.push(ir::Instr::Extend(value, from_sign, *to_ty))
            }
            UnaryCast::Truncate => {
                assert!(to_ty.is_i());
                builder.push(ir::Instr::Truncate(value, *to_ty))
            }
        };
    }
}
