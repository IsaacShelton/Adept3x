use crate::{
    ir::{self, FloatOrSign, Instruction, IntegerSign},
    llvm_backend::{
        backend_type::{get_function_type, to_backend_type},
        builder::{Builder, PhiRelocation},
        ctx::BackendCtx,
        error::BackendError,
        value_catalog::ValueCatalog,
        values::build_value,
    },
    resolved::FloatOrInteger,
};
use cstr::cstr;
use llvm_sys::{
    core::*,
    prelude::{LLVMBasicBlockRef, LLVMValueRef},
    target::LLVMABISizeOfType,
    LLVMIntPredicate::*,
    LLVMRealPredicate::*,
};
use std::{cell::OnceCell, ptr::null_mut};

pub unsafe fn create_function_block(
    ctx: &BackendCtx,
    value_catalog: &mut ValueCatalog,
    overflow_basicblock: &OnceCell<LLVMBasicBlockRef>,
    builder: &Builder,
    ir_basicblock_id: usize,
    ir_basicblock: &ir::BasicBlock,
    mut llvm_basicblock: LLVMBasicBlockRef,
    function_skeleton: LLVMValueRef,
    basicblocks: &[(usize, &ir::BasicBlock, LLVMBasicBlockRef)],
) -> Result<(), BackendError> {
    LLVMPositionBuilderAtEnd(builder.get(), llvm_basicblock);

    for instruction in ir_basicblock.iter() {
        let result = match instruction {
            Instruction::Return(value) => {
                let _ = LLVMBuildRet(
                    builder.get(),
                    value.as_ref().map_or_else(
                        || Ok(null_mut()),
                        |value| build_value(ctx, value_catalog, &builder, value),
                    )?,
                );
                None
            }
            Instruction::Alloca(ir_type) => Some(LLVMBuildAlloca(
                builder.get(),
                to_backend_type(ctx.for_making_type(), ir_type)?,
                cstr!("").as_ptr(),
            )),
            Instruction::Malloc(ir_type) => {
                let backend_type = to_backend_type(ctx.for_making_type(), ir_type)?;
                Some(LLVMBuildMalloc(
                    builder.get(),
                    backend_type,
                    cstr!("").as_ptr(),
                ))
            }
            Instruction::MallocArray(ir_type, count) => {
                let backend_type = to_backend_type(ctx.for_making_type(), ir_type)?;
                let count = build_value(ctx, value_catalog, builder, count)?;
                Some(LLVMBuildArrayMalloc(
                    builder.get(),
                    backend_type,
                    count,
                    cstr!("").as_ptr(),
                ))
            }
            Instruction::Free(value) => {
                let backend_value = build_value(ctx, value_catalog, builder, value)?;
                Some(LLVMBuildFree(builder.get(), backend_value))
            }
            Instruction::SizeOf(ir_type) => {
                let backend_type = to_backend_type(ctx.for_making_type(), ir_type)?;
                let size = LLVMABISizeOfType(ctx.target_data.get(), backend_type);
                Some(LLVMConstInt(LLVMInt64Type(), size, false.into()))
            }
            Instruction::Parameter(index) => Some(LLVMGetParam(function_skeleton, *index)),
            Instruction::GlobalVariable(global_ref) => Some(
                *ctx.globals
                    .get(global_ref)
                    .expect("referenced global to exist"),
            ),
            Instruction::Store(store) => {
                let source = build_value(ctx, value_catalog, builder, &store.new_value)?;
                let destination = build_value(ctx, value_catalog, builder, &store.destination)?;
                let _ = LLVMBuildStore(builder.get(), source, destination);
                None
            }
            Instruction::Load((value, ir_type)) => {
                let pointer = build_value(ctx, value_catalog, builder, value)?;
                let llvm_type = to_backend_type(ctx.for_making_type(), ir_type)?;
                Some(LLVMBuildLoad2(
                    builder.get(),
                    llvm_type,
                    pointer,
                    cstr!("").as_ptr(),
                ))
            }
            Instruction::Call(call) => {
                let function_type = get_function_type(
                    ctx.for_making_type(),
                    ctx.ir_module
                        .functions
                        .get(&call.function)
                        .expect("callee to exist"),
                )?;

                let function_value = *ctx
                    .func_skeletons
                    .get(&call.function)
                    .expect("ir function to exist");

                let mut arguments = call
                    .arguments
                    .iter()
                    .map(|argument| build_value(ctx, value_catalog, builder, argument))
                    .collect::<Result<Vec<LLVMValueRef>, _>>()?;

                Some(LLVMBuildCall2(
                    builder.get(),
                    function_type,
                    function_value,
                    arguments.as_mut_ptr(),
                    arguments.len().try_into().unwrap(),
                    cstr!("").as_ptr(),
                ))
            }
            Instruction::Add(operands, mode) => {
                let (left, right) = build_binary_operands(ctx, value_catalog, builder, operands)?;

                Some(match mode {
                    FloatOrInteger::Integer => {
                        LLVMBuildAdd(builder.get(), left, right, cstr!("").as_ptr())
                    }
                    FloatOrInteger::Float => {
                        LLVMBuildFAdd(builder.get(), left, right, cstr!("").as_ptr())
                    }
                })
            }
            Instruction::Checked(operation, operands) => {
                let (left, right) = build_binary_operands(ctx, value_catalog, builder, operands)?;

                let (expect_i1_fn, expect_i1_fn_type) = ctx.intrinsics.expect_i1();
                let (overflow_panic_fn, overflow_panic_fn_type) = ctx.intrinsics.on_overflow();

                let mut arguments = [left, right];

                let (fn_value, fn_type) = ctx.intrinsics.overflow_operation(&operation);

                let info = LLVMBuildCall2(
                    builder.get(),
                    fn_type,
                    fn_value,
                    arguments.as_mut_ptr(),
                    arguments.len().try_into().unwrap(),
                    cstr!("").as_ptr(),
                );

                // `result = info.0`
                // `overflow = info.1`
                let result = LLVMBuildExtractValue(builder.get(), info, 0, cstr!("").as_ptr());
                let overflowed = LLVMBuildExtractValue(builder.get(), info, 1, cstr!("").as_ptr());

                // `llvm.expect.i1(overflowed, false)`
                let mut arguments = [
                    overflowed,
                    LLVMConstInt(LLVMInt1Type(), false.into(), false.into()),
                ];
                LLVMBuildCall2(
                    builder.get(),
                    expect_i1_fn_type,
                    expect_i1_fn,
                    arguments.as_mut_ptr(),
                    arguments.len().try_into().unwrap(),
                    cstr!("").as_ptr(),
                );

                let overflow_basicblock = *overflow_basicblock.get_or_init(|| {
                    let overflow_basicblock =
                        LLVMAppendBasicBlock(function_skeleton, cstr!("").as_ptr());
                    let mut args = [];

                    LLVMPositionBuilderAtEnd(builder.get(), overflow_basicblock);
                    LLVMBuildCall2(
                        builder.get(),
                        overflow_panic_fn_type,
                        overflow_panic_fn,
                        args.as_mut_ptr(),
                        args.len().try_into().unwrap(),
                        cstr!("").as_ptr(),
                    );
                    LLVMBuildUnreachable(builder.get());
                    LLVMPositionBuilderAtEnd(builder.get(), llvm_basicblock);
                    overflow_basicblock
                });

                // Break to either "ok" basicblock or "overflow occurred" basicblock
                let ok_basicblock = LLVMAppendBasicBlock(function_skeleton, cstr!("").as_ptr());
                LLVMBuildCondBr(
                    builder.get(),
                    overflowed,
                    overflow_basicblock,
                    ok_basicblock,
                );

                // Switch over to new continuation basicblock
                llvm_basicblock = ok_basicblock;
                LLVMPositionBuilderAtEnd(builder.get(), llvm_basicblock);
                Some(result)
            }
            Instruction::Subtract(operands, mode) => {
                let (left, right) = build_binary_operands(ctx, value_catalog, builder, operands)?;

                Some(match mode {
                    FloatOrInteger::Integer => {
                        LLVMBuildSub(builder.get(), left, right, cstr!("").as_ptr())
                    }
                    FloatOrInteger::Float => {
                        LLVMBuildFSub(builder.get(), left, right, cstr!("").as_ptr())
                    }
                })
            }
            Instruction::Multiply(operands, mode) => {
                let (left, right) = build_binary_operands(ctx, value_catalog, builder, operands)?;

                Some(match mode {
                    FloatOrInteger::Integer => {
                        LLVMBuildMul(builder.get(), left, right, cstr!("").as_ptr())
                    }
                    FloatOrInteger::Float => {
                        LLVMBuildFMul(builder.get(), left, right, cstr!("").as_ptr())
                    }
                })
            }
            Instruction::Divide(operands, sign) => {
                let (left, right) = build_binary_operands(ctx, value_catalog, builder, operands)?;

                Some(match sign {
                    FloatOrSign::Integer(IntegerSign::Signed) => {
                        LLVMBuildSDiv(builder.get(), left, right, cstr!("").as_ptr())
                    }
                    FloatOrSign::Integer(IntegerSign::Unsigned) => {
                        LLVMBuildUDiv(builder.get(), left, right, cstr!("").as_ptr())
                    }
                    FloatOrSign::Float => {
                        LLVMBuildFDiv(builder.get(), left, right, cstr!("").as_ptr())
                    }
                })
            }
            Instruction::Modulus(operands, sign) => {
                let (left, right) = build_binary_operands(ctx, value_catalog, builder, operands)?;

                Some(match sign {
                    FloatOrSign::Integer(IntegerSign::Signed) => {
                        LLVMBuildSRem(builder.get(), left, right, cstr!("").as_ptr())
                    }
                    FloatOrSign::Integer(IntegerSign::Unsigned) => {
                        LLVMBuildURem(builder.get(), left, right, cstr!("").as_ptr())
                    }
                    FloatOrSign::Float => {
                        LLVMBuildFRem(builder.get(), left, right, cstr!("").as_ptr())
                    }
                })
            }
            Instruction::Equals(operands, mode) => {
                let (left, right) = build_binary_operands(ctx, value_catalog, builder, operands)?;

                Some(match mode {
                    FloatOrInteger::Integer => {
                        LLVMBuildICmp(builder.get(), LLVMIntEQ, left, right, cstr!("").as_ptr())
                    }
                    FloatOrInteger::Float => {
                        LLVMBuildFCmp(builder.get(), LLVMRealOEQ, left, right, cstr!("").as_ptr())
                    }
                })
            }
            Instruction::NotEquals(operands, mode) => {
                let (left, right) = build_binary_operands(ctx, value_catalog, builder, operands)?;

                Some(match mode {
                    FloatOrInteger::Integer => {
                        LLVMBuildICmp(builder.get(), LLVMIntNE, left, right, cstr!("").as_ptr())
                    }
                    FloatOrInteger::Float => {
                        LLVMBuildFCmp(builder.get(), LLVMRealONE, left, right, cstr!("").as_ptr())
                    }
                })
            }
            Instruction::LessThan(operands, mode) => {
                let (left, right) = build_binary_operands(ctx, value_catalog, builder, operands)?;

                Some(match mode {
                    FloatOrSign::Integer(sign) => LLVMBuildICmp(
                        builder.get(),
                        match sign {
                            IntegerSign::Signed => LLVMIntSLT,
                            IntegerSign::Unsigned => LLVMIntULT,
                        },
                        left,
                        right,
                        cstr!("").as_ptr(),
                    ),
                    FloatOrSign::Float => {
                        LLVMBuildFCmp(builder.get(), LLVMRealOLT, left, right, cstr!("").as_ptr())
                    }
                })
            }
            Instruction::LessThanEq(operands, mode) => {
                let (left, right) = build_binary_operands(ctx, value_catalog, builder, operands)?;

                Some(match mode {
                    FloatOrSign::Integer(sign) => LLVMBuildICmp(
                        builder.get(),
                        match sign {
                            IntegerSign::Signed => LLVMIntSLE,
                            IntegerSign::Unsigned => LLVMIntULE,
                        },
                        left,
                        right,
                        cstr!("").as_ptr(),
                    ),
                    FloatOrSign::Float => {
                        LLVMBuildFCmp(builder.get(), LLVMRealOLE, left, right, cstr!("").as_ptr())
                    }
                })
            }
            Instruction::GreaterThan(operands, mode) => {
                let (left, right) = build_binary_operands(ctx, value_catalog, builder, operands)?;

                Some(match mode {
                    FloatOrSign::Integer(sign) => LLVMBuildICmp(
                        builder.get(),
                        match sign {
                            IntegerSign::Signed => LLVMIntSGT,
                            IntegerSign::Unsigned => LLVMIntUGT,
                        },
                        left,
                        right,
                        cstr!("").as_ptr(),
                    ),
                    FloatOrSign::Float => {
                        LLVMBuildFCmp(builder.get(), LLVMRealOGT, left, right, cstr!("").as_ptr())
                    }
                })
            }
            Instruction::GreaterThanEq(operands, mode) => {
                let (left, right) = build_binary_operands(ctx, value_catalog, builder, operands)?;

                Some(match mode {
                    FloatOrSign::Integer(sign) => LLVMBuildICmp(
                        builder.get(),
                        match sign {
                            IntegerSign::Signed => LLVMIntSGE,
                            IntegerSign::Unsigned => LLVMIntUGE,
                        },
                        left,
                        right,
                        cstr!("").as_ptr(),
                    ),
                    FloatOrSign::Float => {
                        LLVMBuildFCmp(builder.get(), LLVMRealOGE, left, right, cstr!("").as_ptr())
                    }
                })
            }
            Instruction::And(operands) | Instruction::BitwiseAnd(operands) => {
                let (left, right) = build_binary_operands(ctx, value_catalog, builder, operands)?;

                Some(LLVMBuildAnd(builder.get(), left, right, cstr!("").as_ptr()))
            }
            Instruction::Or(operands) | Instruction::BitwiseOr(operands) => {
                let (left, right) = build_binary_operands(ctx, value_catalog, builder, operands)?;

                Some(LLVMBuildOr(builder.get(), left, right, cstr!("").as_ptr()))
            }
            Instruction::BitwiseXor(operands) => {
                let (left, right) = build_binary_operands(ctx, value_catalog, builder, operands)?;

                Some(LLVMBuildXor(builder.get(), left, right, cstr!("").as_ptr()))
            }
            Instruction::LeftShift(operands) => {
                let (left, right) = build_binary_operands(ctx, value_catalog, builder, operands)?;

                Some(LLVMBuildShl(builder.get(), left, right, cstr!("").as_ptr()))
            }
            Instruction::RightShift(operands) => {
                let (left, right) = build_binary_operands(ctx, value_catalog, builder, operands)?;

                Some(LLVMBuildAShr(
                    builder.get(),
                    left,
                    right,
                    cstr!("").as_ptr(),
                ))
            }
            Instruction::LogicalRightShift(operands) => {
                let (left, right) = build_binary_operands(ctx, value_catalog, builder, operands)?;

                Some(LLVMBuildLShr(
                    builder.get(),
                    left,
                    right,
                    cstr!("").as_ptr(),
                ))
            }
            Instruction::Bitcast(value, ir_type) => {
                let value = build_value(ctx, value_catalog, builder, value)?;
                let backend_type = to_backend_type(ctx.for_making_type(), ir_type)?;
                Some(LLVMBuildBitCast(
                    builder.get(),
                    value,
                    backend_type,
                    cstr!("").as_ptr(),
                ))
            }
            Instruction::ZeroExtend(value, ir_type) => {
                let value = build_value(ctx, value_catalog, builder, value)?;
                let backend_type = to_backend_type(ctx.for_making_type(), ir_type)?;
                Some(LLVMBuildZExt(
                    builder.get(),
                    value,
                    backend_type,
                    cstr!("").as_ptr(),
                ))
            }
            Instruction::SignExtend(value, ir_type) => {
                let value = build_value(ctx, value_catalog, builder, value)?;
                let backend_type = to_backend_type(ctx.for_making_type(), ir_type)?;
                Some(LLVMBuildSExt(
                    builder.get(),
                    value,
                    backend_type,
                    cstr!("").as_ptr(),
                ))
            }
            Instruction::FloatExtend(value, ir_type) => {
                let value = build_value(ctx, value_catalog, builder, value)?;
                let backend_type = to_backend_type(ctx.for_making_type(), ir_type)?;
                Some(LLVMBuildFPExt(
                    builder.get(),
                    value,
                    backend_type,
                    cstr!("").as_ptr(),
                ))
            }
            Instruction::Truncate(value, ir_type) => {
                let value = build_value(ctx, value_catalog, builder, value)?;
                let backend_type = to_backend_type(ctx.for_making_type(), ir_type)?;
                Some(LLVMBuildTrunc(
                    builder.get(),
                    value,
                    backend_type,
                    cstr!("").as_ptr(),
                ))
            }
            Instruction::TruncateFloat(value, ir_type) => {
                let value = build_value(ctx, value_catalog, builder, value)?;
                let backend_type = to_backend_type(ctx.for_making_type(), ir_type)?;
                Some(LLVMBuildFPTrunc(
                    builder.get(),
                    value,
                    backend_type,
                    cstr!("").as_ptr(),
                ))
            }
            Instruction::Member {
                subject_pointer,
                struct_type: ir_struct_type,
                index,
            } => {
                let pointer = build_value(ctx, value_catalog, builder, subject_pointer)?;

                let backend_struct_type = match ir_struct_type {
                    ir::Type::Structure(_) | ir::Type::AnonymousComposite(_) => {
                        to_backend_type(ctx.for_making_type(), ir_struct_type)?
                    }
                    _ => return Err("cannot use member instruction on non-structure".into()),
                };

                let mut indices = [
                    LLVMConstInt(LLVMInt32Type(), 0, true.into()),
                    LLVMConstInt(LLVMInt32Type(), (*index).try_into().unwrap(), true.into()),
                ];

                Some(LLVMBuildGEP2(
                    builder.get(),
                    backend_struct_type,
                    pointer,
                    indices.as_mut_ptr(),
                    indices.len().try_into().unwrap(),
                    cstr!("").as_ptr(),
                ))
            }
            Instruction::ArrayAccess {
                subject_pointer,
                item_type: ir_item_type,
                index,
            } => {
                let pointer = build_value(ctx, value_catalog, builder, subject_pointer)?;
                let index_value = build_value(ctx, value_catalog, builder, index)?;

                let backend_item_type = to_backend_type(ctx.for_making_type(), ir_item_type)?;
                let mut indices = [index_value];

                Some(LLVMBuildGEP2(
                    builder.get(),
                    backend_item_type,
                    pointer,
                    indices.as_mut_ptr(),
                    indices.len().try_into().unwrap(),
                    cstr!("").as_ptr(),
                ))
            }
            Instruction::StructureLiteral(ir_type, values) => {
                let backend_type = to_backend_type(ctx.for_making_type(), ir_type)?;
                let mut literal = LLVMGetUndef(backend_type);

                for (index, value) in values.iter().enumerate() {
                    let backend_value = build_value(ctx, value_catalog, builder, value)?;

                    literal = LLVMBuildInsertValue(
                        builder.get(),
                        literal,
                        backend_value,
                        index.try_into().unwrap(),
                        cstr!("").as_ptr(),
                    );
                }

                Some(literal)
            }
            Instruction::IsZero(inner) => {
                let value = build_value(ctx, value_catalog, builder, inner)?;
                Some(LLVMBuildIsNull(builder.get(), value, cstr!("").as_ptr()))
            }
            Instruction::IsNotZero(inner) => {
                let value = build_value(ctx, value_catalog, builder, inner)?;
                Some(LLVMBuildIsNotNull(builder.get(), value, cstr!("").as_ptr()))
            }
            Instruction::Negate(inner) => {
                let value = build_value(ctx, value_catalog, builder, inner)?;
                Some(LLVMBuildNeg(builder.get(), value, cstr!("").as_ptr()))
            }
            Instruction::NegateFloat(inner) => {
                let value = build_value(ctx, value_catalog, builder, inner)?;
                Some(LLVMBuildFNeg(builder.get(), value, cstr!("").as_ptr()))
            }
            Instruction::BitComplement(inner) => {
                let value = build_value(ctx, value_catalog, builder, inner)?;
                Some(LLVMBuildNot(builder.get(), value, cstr!("").as_ptr()))
            }
            Instruction::Break(break_info) => {
                let (_, _, backend_block) = basicblocks
                    .get(break_info.basicblock_id)
                    .expect("referenced basicblock to exist");
                Some(LLVMBuildBr(builder.get(), *backend_block))
            }
            Instruction::ConditionalBreak(condition, break_info) => {
                let value = build_value(ctx, value_catalog, builder, condition)?;

                let (_, _, true_backend_block) = basicblocks
                    .get(break_info.true_basicblock_id)
                    .expect("referenced basicblock to exist");

                let (_, _, false_backend_block) = basicblocks
                    .get(break_info.false_basicblock_id)
                    .expect("referenced basicblock to exist");

                Some(LLVMBuildCondBr(
                    builder.get(),
                    value,
                    *true_backend_block,
                    *false_backend_block,
                ))
            }
            Instruction::Phi(phi) => {
                let backend_type = to_backend_type(ctx.for_making_type(), &phi.ir_type)?;
                let phi_node = LLVMBuildPhi(builder.get(), backend_type, cstr!("").as_ptr());

                builder.add_phi_relocation(PhiRelocation {
                    phi_node,
                    incoming: phi.incoming.clone(),
                });

                Some(phi_node)
            }
        };

        value_catalog.push(ir_basicblock_id, result);
    }

    Ok(())
}

unsafe fn build_binary_operands(
    ctx: &BackendCtx<'_>,
    value_catalog: &ValueCatalog,
    builder: &Builder,
    operands: &ir::BinaryOperands,
) -> Result<(LLVMValueRef, LLVMValueRef), BackendError> {
    let left = build_value(ctx, value_catalog, builder, &operands.left)?;
    let right = build_value(ctx, value_catalog, builder, &operands.right)?;
    Ok((left, right))
}
