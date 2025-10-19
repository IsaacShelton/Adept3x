use crate::{
    Continuation, Execution, ExecutionCtx, Executor, Suspend,
    conform::UnaryCast,
    execution::lower::{LowerType, function_body::ir_builder::IrBuilder},
    ir,
    module_graph::ModuleView,
    repr::Compiler,
    sub_task::SubTask,
};
use diagnostics::ErrorDiagnostic;
use num_bigint::BigInt;
use num_traits::ToPrimitive;
use primitives::{FloatSize, IntegerBits, IntegerSign};
use source_files::Source;

#[derive(Clone)]
pub struct PerformUnaryCast<'env> {
    view: &'env ModuleView<'env>,
    compiler: &'env Compiler<'env>,
    from: ir::Value<'env>,
    to: ir::Type<'env>,
    unary_cast: Option<&'env UnaryCast<'env>>,
    source: Source,
    lowered_type: Suspend<'env, ir::Type<'env>>,
}

impl<'env> PerformUnaryCast<'env> {
    pub fn new(
        view: &'env ModuleView<'env>,
        compiler: &'env Compiler<'env>,
        from: ir::Value<'env>,
        to: ir::Type<'env>,
        unary_cast: Option<&'env UnaryCast<'env>>,
        source: Source,
    ) -> Self {
        Self {
            view,
            compiler,
            from,
            to,
            unary_cast,
            source,
            lowered_type: None,
        }
    }
}

impl<'env> SubTask<'env> for PerformUnaryCast<'env> {
    type SubArtifact<'a>
        = ir::Value<'env>
    where
        Self: 'a,
        'env: 'a;

    type UserData<'a>
        = &'a mut IrBuilder<'env>
    where
        Self: 'a,
        'env: 'a;

    fn execute_sub_task<'a, 'ctx>(
        &'a mut self,
        executor: &'a Executor<'env>,
        ctx: &'ctx mut ExecutionCtx<'env>,
        builder: Self::UserData<'a>,
    ) -> Result<
        Self::SubArtifact<'a>,
        Result<impl FnOnce(Execution<'env>) -> Continuation<'env> + 'env, ErrorDiagnostic>,
    > {
        let cast_failed = |_| {
            Err(ErrorDiagnostic::ice(
                format!("Failed to perform cast to {:?}", self.to),
                Some(self.source),
            ))
        };

        loop {
            let Some(cast) = self.unary_cast.take() else {
                return Ok(self.from);
            };

            use IntegerBits::*;
            use IntegerSign::*;

            self.from = match *cast {
                UnaryCast::SpecializeBoolean(value) => ir::Literal::Boolean(value).into(),
                UnaryCast::SpecializeInteger(value) => match &self.to {
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
                    _ => {
                        return Err(Err(ErrorDiagnostic::ice(
                            format!("Cannot specialize integer for type {:?}", self.to),
                            Some(self.source),
                        )));
                    }
                },
                UnaryCast::SpecializeFloat(not_nan) => todo!("specialize float"),
                UnaryCast::SpecializePointerOuter(unaliased_type) => {
                    todo!("specialize pointer outer")
                }
                UnaryCast::SpecializeAsciiChar(_) => todo!("specialize ascii char"),
                UnaryCast::Dereference { after_deref, then } => {
                    // Continue processing this cast after suspending
                    self.unary_cast = Some(cast);

                    let Some(after_deref) = executor.demand(self.lowered_type) else {
                        return suspend_from_sub_task!(
                            self.lowered_type,
                            executor.request(LowerType::new(
                                self.view,
                                &self.compiler,
                                after_deref.0
                            )),
                            ctx
                        );
                    };

                    // Proccess any inner unary cast next
                    self.unary_cast = then;

                    builder.push(ir::Instr::Load {
                        pointer: self.from,
                        pointee: after_deref,
                    })
                }
                UnaryCast::Extend(from_sign) => {
                    assert!(self.to.is_i());
                    builder.push(ir::Instr::Extend(self.from, from_sign, self.to))
                }
                UnaryCast::Truncate => {
                    assert!(self.to.is_i());
                    builder.push(ir::Instr::Truncate(self.from, self.to))
                }
            };

            self.lowered_type = None;
        }
    }
}
