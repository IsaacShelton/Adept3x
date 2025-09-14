use super::{
    body::{BasicBlockInfo, FnCtx},
    function_type::FunctionType,
    helpers::emit_load_of_scalar,
    params_mapping::Param,
};
use crate::{
    build_llvm_ir::{
        abi::{
            abi_function::{ABIFunction, ABIParam},
            abi_type::{
                ABITypeKind, CoerceAndExpand, is_padding_for_coerce_expand,
                kinds::{TypeExpansion, get_type_expansion},
            },
            has_scalar_evaluation_kind, is_promotable_integer_type_for_abi,
        },
        address::Address,
        backend_type::{get_abi_function_type, get_unabi_function_type, to_backend_type},
        builder::{Builder, PhiRelocation, Volatility},
        ctx::BackendCtx,
        functions::{
            helpers::{
                build_coerced_load, build_coerced_store, build_mem_tmp, build_mem_tmp_without_cast,
                build_tmp_alloca_address, emit_address_at_offset,
            },
            params_mapping::ParamsMapping,
        },
        llvm_type_ref_ext::LLVMTypeRefExt,
        llvm_value_ref_ext::LLVMValueRefExt,
        value_catalog::ValueCatalog,
        values::build_value,
    },
    ir::{self, Call, Instr},
    target_layout::TargetLayout,
};
use ast::SizeOfMode;
use data_units::{BitUnits, ByteUnits};
use diagnostics::ErrorDiagnostic;
use itertools::izip;
use llvm_sys::{
    LLVMIntPredicate::*,
    LLVMRealPredicate::{self, *},
    core::*,
    prelude::{LLVMTypeRef, LLVMValueRef},
};
use primitives::{FloatOrInteger, FloatOrSign, FloatSize, IntegerBits, IntegerSign};
use std::{borrow::Cow, ptr::null_mut, sync::atomic};
use target::Target;

pub unsafe fn create_function_block<'env>(
    ctx: &BackendCtx<'_, 'env>,
    builder: &Builder<'env>,
    basicblock: &BasicBlockInfo<'env>,
    function_skeleton: LLVMValueRef,
    basicblocks: &[BasicBlockInfo<'env>],
    fn_ctx: &FnCtx,
    value_catalog: &mut ValueCatalog,
) -> Result<(), ErrorDiagnostic> {
    let &BasicBlockInfo {
        id: ir_basicblock_id,
        ir_basicblock,
        mut llvm_basicblock,
    } = basicblock;

    let ir_basicblock_id = ir_basicblock_id;

    builder.position(llvm_basicblock);

    for instruction in ir_basicblock.instructions.iter() {
        let result = match instruction {
            Instr::Return(value) => {
                // If the function has an ABI prologue and epilogue, then return using them.
                // Otherwise, return plainly.

                if let Some((prologue, epilogue)) =
                    fn_ctx.prologue.as_ref().zip(fn_ctx.epilogue.as_ref())
                {
                    if let Some(value) = value.as_ref() {
                        let return_value = build_value(ctx, value_catalog, builder, value)?;

                        if let Some(return_location) = &prologue.return_location {
                            builder.store(return_value, &return_location.return_value_address);
                        }
                    }

                    builder.br(epilogue.llvm_basicblock);
                } else {
                    LLVMBuildRet(
                        builder.get(),
                        value.as_ref().map_or_else(
                            || Ok(null_mut()),
                            |value| build_value(ctx, value_catalog, builder, value),
                        )?,
                    );
                }

                None
            }
            Instr::Alloca(ir_type) => Some(LLVMBuildAlloca(
                builder.get(),
                to_backend_type(ctx.for_making_type(), ir_type)?,
                c"".as_ptr(),
            )),
            Instr::Malloc(ir_type) => {
                let backend_type = to_backend_type(ctx.for_making_type(), ir_type)?;
                Some(LLVMBuildMalloc(builder.get(), backend_type, c"".as_ptr()))
            }
            Instr::MallocArray(ir_type, count) => {
                let backend_type = to_backend_type(ctx.for_making_type(), ir_type)?;
                let count = build_value(ctx, value_catalog, builder, count)?;
                Some(LLVMBuildArrayMalloc(
                    builder.get(),
                    backend_type,
                    count,
                    c"".as_ptr(),
                ))
            }
            Instr::Free(value) => {
                let backend_value = build_value(ctx, value_catalog, builder, value)?;
                Some(LLVMBuildFree(builder.get(), backend_value))
            }
            Instr::SizeOf(ir_type, mode) => {
                let backend_type = to_backend_type(ctx.for_making_type(), ir_type)?;
                let size = ctx.target_data.abi_size_of_type(backend_type);

                match mode {
                    Some(SizeOfMode::Compilation) => {
                        return Err(ErrorDiagnostic::plain(
                            "Cannot use result of sizeof<\"compilation\", T> at runtime",
                        ));
                    }
                    Some(SizeOfMode::Target) | None => Some(LLVMValueRef::new_u64(size.bytes())),
                }
            }
            Instr::Parameter(index) => Some(if let Some(prologue) = fn_ctx.prologue.as_ref() {
                let param_value = prologue
                    .param_values
                    .get((*index).try_into().unwrap())
                    .expect("parameter to exist");

                param_value.value()
            } else {
                LLVMGetParam(function_skeleton, *index)
            }),
            Instr::GlobalVariable(global_ref) => Some(
                *ctx.globals
                    .get(global_ref)
                    .expect("referenced global to exist"),
            ),
            Instr::Store(store) => {
                let source = build_value(ctx, value_catalog, builder, &store.new_value)?;
                let destination = build_value(ctx, value_catalog, builder, &store.destination)?;
                LLVMBuildStore(builder.get(), source, destination);
                None
            }
            Instr::Load {
                pointer,
                pointee: pointee_ir_type,
            } => {
                let pointer = build_value(ctx, value_catalog, builder, pointer)?;
                let llvm_type = to_backend_type(ctx.for_making_type(), pointee_ir_type)?;
                Some(LLVMBuildLoad2(
                    builder.get(),
                    llvm_type,
                    pointer,
                    c"".as_ptr(),
                ))
            }
            Instr::Call(call) => Some(emit_call(ctx, builder, call, fn_ctx, value_catalog)?),
            Instr::Add(operands, mode) => {
                let (left, right) = build_binary_operands(ctx, value_catalog, builder, operands)?;

                Some(match mode {
                    FloatOrInteger::Integer => {
                        LLVMBuildAdd(builder.get(), left, right, c"".as_ptr())
                    }
                    FloatOrInteger::Float => {
                        LLVMBuildFAdd(builder.get(), left, right, c"".as_ptr())
                    }
                })
            }
            Instr::Checked(operation, operands) => {
                let (left, right) = build_binary_operands(ctx, value_catalog, builder, operands)?;

                let (expect_i1_fn, expect_i1_fn_type) = ctx.intrinsics.expect_i1();
                let (overflow_panic_fn, overflow_panic_fn_type) =
                    ctx.intrinsics.on_overflow(builder);

                let mut arguments = [left, right];

                let (fn_value, fn_type) = ctx.intrinsics.overflow_operation(operation);

                let info = LLVMBuildCall2(
                    builder.get(),
                    fn_type,
                    fn_value,
                    arguments.as_mut_ptr(),
                    arguments.len().try_into().unwrap(),
                    c"".as_ptr(),
                );

                // `result = info.0`
                // `overflow = info.1`
                let result = LLVMBuildExtractValue(builder.get(), info, 0, c"".as_ptr());
                let overflowed = LLVMBuildExtractValue(builder.get(), info, 1, c"".as_ptr());

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
                    c"".as_ptr(),
                );

                let overflow_basicblock = *fn_ctx.overflow_basicblock.get_or_init(|| {
                    let overflow_basicblock = LLVMAppendBasicBlock(function_skeleton, c"".as_ptr());
                    let mut args = [];

                    builder.position(overflow_basicblock);

                    LLVMBuildCall2(
                        builder.get(),
                        overflow_panic_fn_type,
                        overflow_panic_fn,
                        args.as_mut_ptr(),
                        args.len().try_into().unwrap(),
                        c"".as_ptr(),
                    );
                    LLVMBuildUnreachable(builder.get());

                    builder.position(llvm_basicblock);
                    overflow_basicblock
                });

                // Break to either "ok" basicblock or "overflow occurred" basicblock
                let ok_basicblock = LLVMAppendBasicBlock(function_skeleton, c"".as_ptr());
                LLVMBuildCondBr(
                    builder.get(),
                    overflowed,
                    overflow_basicblock,
                    ok_basicblock,
                );

                // Switch over to new continuation basicblock
                builder.position(ok_basicblock);
                llvm_basicblock = ok_basicblock;
                Some(result)
            }
            Instr::Subtract(operands, mode) => {
                let (left, right) = build_binary_operands(ctx, value_catalog, builder, operands)?;

                Some(match mode {
                    FloatOrInteger::Integer => {
                        LLVMBuildSub(builder.get(), left, right, c"".as_ptr())
                    }
                    FloatOrInteger::Float => {
                        LLVMBuildFSub(builder.get(), left, right, c"".as_ptr())
                    }
                })
            }
            Instr::Multiply(operands, mode) => {
                let (left, right) = build_binary_operands(ctx, value_catalog, builder, operands)?;

                Some(match mode {
                    FloatOrInteger::Integer => {
                        LLVMBuildMul(builder.get(), left, right, c"".as_ptr())
                    }
                    FloatOrInteger::Float => {
                        LLVMBuildFMul(builder.get(), left, right, c"".as_ptr())
                    }
                })
            }
            Instr::Divide(operands, sign) => {
                let (left, right) = build_binary_operands(ctx, value_catalog, builder, operands)?;

                Some(match sign {
                    FloatOrSign::Integer(IntegerSign::Signed) => {
                        LLVMBuildSDiv(builder.get(), left, right, c"".as_ptr())
                    }
                    FloatOrSign::Integer(IntegerSign::Unsigned) => {
                        LLVMBuildUDiv(builder.get(), left, right, c"".as_ptr())
                    }
                    FloatOrSign::Float => LLVMBuildFDiv(builder.get(), left, right, c"".as_ptr()),
                })
            }
            Instr::Modulus(operands, sign) => {
                let (left, right) = build_binary_operands(ctx, value_catalog, builder, operands)?;

                Some(match sign {
                    FloatOrSign::Integer(IntegerSign::Signed) => {
                        LLVMBuildSRem(builder.get(), left, right, c"".as_ptr())
                    }
                    FloatOrSign::Integer(IntegerSign::Unsigned) => {
                        LLVMBuildURem(builder.get(), left, right, c"".as_ptr())
                    }
                    FloatOrSign::Float => LLVMBuildFRem(builder.get(), left, right, c"".as_ptr()),
                })
            }
            Instr::Equals(operands, mode) => {
                let (left, right) = build_binary_operands(ctx, value_catalog, builder, operands)?;

                Some(match mode {
                    FloatOrInteger::Integer => {
                        LLVMBuildICmp(builder.get(), LLVMIntEQ, left, right, c"".as_ptr())
                    }
                    FloatOrInteger::Float => {
                        LLVMBuildFCmp(builder.get(), LLVMRealOEQ, left, right, c"".as_ptr())
                    }
                })
            }
            Instr::NotEquals(operands, mode) => {
                let (left, right) = build_binary_operands(ctx, value_catalog, builder, operands)?;

                Some(match mode {
                    FloatOrInteger::Integer => {
                        LLVMBuildICmp(builder.get(), LLVMIntNE, left, right, c"".as_ptr())
                    }
                    FloatOrInteger::Float => {
                        LLVMBuildFCmp(builder.get(), LLVMRealONE, left, right, c"".as_ptr())
                    }
                })
            }
            Instr::LessThan(operands, mode) => {
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
                        c"".as_ptr(),
                    ),
                    FloatOrSign::Float => {
                        LLVMBuildFCmp(builder.get(), LLVMRealOLT, left, right, c"".as_ptr())
                    }
                })
            }
            Instr::LessThanEq(operands, mode) => {
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
                        c"".as_ptr(),
                    ),
                    FloatOrSign::Float => {
                        LLVMBuildFCmp(builder.get(), LLVMRealOLE, left, right, c"".as_ptr())
                    }
                })
            }
            Instr::GreaterThan(operands, mode) => {
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
                        c"".as_ptr(),
                    ),
                    FloatOrSign::Float => {
                        LLVMBuildFCmp(builder.get(), LLVMRealOGT, left, right, c"".as_ptr())
                    }
                })
            }
            Instr::GreaterThanEq(operands, mode) => {
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
                        c"".as_ptr(),
                    ),
                    FloatOrSign::Float => {
                        LLVMBuildFCmp(builder.get(), LLVMRealOGE, left, right, c"".as_ptr())
                    }
                })
            }
            Instr::And(operands) | Instr::BitwiseAnd(operands) => {
                let (left, right) = build_binary_operands(ctx, value_catalog, builder, operands)?;

                Some(LLVMBuildAnd(builder.get(), left, right, c"".as_ptr()))
            }
            Instr::Or(operands) | Instr::BitwiseOr(operands) => {
                let (left, right) = build_binary_operands(ctx, value_catalog, builder, operands)?;

                Some(LLVMBuildOr(builder.get(), left, right, c"".as_ptr()))
            }
            Instr::BitwiseXor(operands) => {
                let (left, right) = build_binary_operands(ctx, value_catalog, builder, operands)?;

                Some(LLVMBuildXor(builder.get(), left, right, c"".as_ptr()))
            }
            Instr::LeftShift(operands) => {
                let (left, right) = build_binary_operands(ctx, value_catalog, builder, operands)?;

                Some(LLVMBuildShl(builder.get(), left, right, c"".as_ptr()))
            }
            Instr::ArithmeticRightShift(operands) => {
                let (left, right) = build_binary_operands(ctx, value_catalog, builder, operands)?;

                Some(LLVMBuildAShr(builder.get(), left, right, c"".as_ptr()))
            }
            Instr::LogicalRightShift(operands) => {
                let (left, right) = build_binary_operands(ctx, value_catalog, builder, operands)?;

                Some(LLVMBuildLShr(builder.get(), left, right, c"".as_ptr()))
            }
            Instr::Bitcast(value, ir_type) => {
                let value = build_value(ctx, value_catalog, builder, value)?;
                let backend_type = to_backend_type(ctx.for_making_type(), ir_type)?;
                Some(builder.bitcast(value, backend_type))
            }
            Instr::Extend(value, sign, ir_type) => {
                let value = build_value(ctx, value_catalog, builder, value)?;
                let backend_type = to_backend_type(ctx.for_making_type(), ir_type)?;

                Some(match sign {
                    IntegerSign::Signed => {
                        LLVMBuildSExt(builder.get(), value, backend_type, c"".as_ptr())
                    }
                    IntegerSign::Unsigned => {
                        LLVMBuildZExt(builder.get(), value, backend_type, c"".as_ptr())
                    }
                })
            }
            Instr::FloatExtend(value, ir_type) => {
                let value = build_value(ctx, value_catalog, builder, value)?;
                let backend_type = to_backend_type(ctx.for_making_type(), ir_type)?;
                Some(LLVMBuildFPExt(
                    builder.get(),
                    value,
                    backend_type,
                    c"".as_ptr(),
                ))
            }
            Instr::Truncate(value, ir_type) => {
                let value = build_value(ctx, value_catalog, builder, value)?;
                let backend_type = to_backend_type(ctx.for_making_type(), ir_type)?;
                Some(LLVMBuildTrunc(
                    builder.get(),
                    value,
                    backend_type,
                    c"".as_ptr(),
                ))
            }
            Instr::TruncateFloat(value, ir_type) => {
                let value = build_value(ctx, value_catalog, builder, value)?;
                let backend_type = to_backend_type(ctx.for_making_type(), ir_type)?;
                Some(LLVMBuildFPTrunc(
                    builder.get(),
                    value,
                    backend_type,
                    c"".as_ptr(),
                ))
            }
            Instr::Member {
                subject_pointer,
                struct_type: ir_struct_type,
                index,
            } => {
                let pointer = build_value(ctx, value_catalog, builder, subject_pointer)?;

                let backend_struct_type = match ir_struct_type {
                    ir::Type::Struct(_) | ir::Type::AnonymousComposite(_) => {
                        to_backend_type(ctx.for_making_type(), ir_struct_type)?
                    }
                    _ => {
                        return Err(ErrorDiagnostic::plain(
                            "Cannot use member instruction on non-structure",
                        ));
                    }
                };

                let mut indices = [
                    LLVMValueRef::new_i32(0),
                    LLVMValueRef::new_i32((*index).try_into().unwrap()),
                ];

                Some(LLVMBuildGEP2(
                    builder.get(),
                    backend_struct_type,
                    pointer,
                    indices.as_mut_ptr(),
                    indices.len().try_into().unwrap(),
                    c"".as_ptr(),
                ))
            }
            Instr::ArrayAccess {
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
                    c"".as_ptr(),
                ))
            }
            Instr::StructLiteral(ir_type, values) => {
                let backend_type = to_backend_type(ctx.for_making_type(), ir_type)?;
                let mut literal = LLVMGetPoison(backend_type);

                for (index, value) in values.iter().enumerate() {
                    let backend_value = build_value(ctx, value_catalog, builder, value)?;

                    literal = LLVMBuildInsertValue(
                        builder.get(),
                        literal,
                        backend_value,
                        index.try_into().unwrap(),
                        c"".as_ptr(),
                    );
                }

                Some(literal)
            }
            Instr::IsZero(inner, floating) => {
                let value = build_value(ctx, value_catalog, builder, inner)?;

                Some(match floating {
                    FloatOrInteger::Integer => LLVMBuildIsNull(builder.get(), value, c"".as_ptr()),
                    FloatOrInteger::Float => LLVMBuildFCmp(
                        builder.get(),
                        LLVMRealPredicate::LLVMRealOEQ,
                        value,
                        LLVMConstNull(LLVMTypeOf(value)),
                        c"".as_ptr(),
                    ),
                })
            }
            Instr::IsNonZero(inner, floating) => {
                let value = build_value(ctx, value_catalog, builder, inner)?;

                Some(match floating {
                    FloatOrInteger::Integer => {
                        LLVMBuildIsNotNull(builder.get(), value, c"".as_ptr())
                    }
                    FloatOrInteger::Float => LLVMBuildFCmp(
                        builder.get(),
                        LLVMRealPredicate::LLVMRealONE,
                        value,
                        LLVMConstNull(LLVMTypeOf(value)),
                        c"".as_ptr(),
                    ),
                })
            }
            Instr::Negate(inner, floating) => {
                let value = build_value(ctx, value_catalog, builder, inner)?;

                Some(match floating {
                    FloatOrInteger::Integer => LLVMBuildNeg(builder.get(), value, c"".as_ptr()),
                    FloatOrInteger::Float => LLVMBuildFNeg(builder.get(), value, c"".as_ptr()),
                })
            }
            Instr::BitComplement(inner) => {
                let value = build_value(ctx, value_catalog, builder, inner)?;
                Some(LLVMBuildNot(builder.get(), value, c"".as_ptr()))
            }
            Instr::Break(break_info) => {
                let backend_block = basicblocks
                    .get(break_info.basicblock_id)
                    .expect("referenced basicblock to exist")
                    .llvm_basicblock;

                Some(LLVMBuildBr(builder.get(), backend_block))
            }
            Instr::ConditionalBreak(condition, break_info) => {
                let value = build_value(ctx, value_catalog, builder, condition)?;

                let true_backend_block = basicblocks
                    .get(break_info.true_basicblock_id)
                    .expect("referenced basicblock to exist")
                    .llvm_basicblock;

                let false_backend_block = basicblocks
                    .get(break_info.false_basicblock_id)
                    .expect("referenced basicblock to exist")
                    .llvm_basicblock;

                Some(LLVMBuildCondBr(
                    builder.get(),
                    value,
                    true_backend_block,
                    false_backend_block,
                ))
            }
            Instr::Phi(phi) => {
                let backend_type = to_backend_type(ctx.for_making_type(), &phi.ir_type)?;
                let phi_node = LLVMBuildPhi(builder.get(), backend_type, c"".as_ptr());

                builder.add_phi_relocation(PhiRelocation {
                    phi_node,
                    incoming: phi.incoming,
                });

                Some(phi_node)
            }
            Instr::InterpreterSyscall(..) => {
                return Err(ErrorDiagnostic::plain(
                    "Cannot use interpreter syscalls in native code",
                ));
            }
            Instr::IntegerToPointer(value, ir_type) => {
                let value = build_value(ctx, value_catalog, builder, value)?;
                let backend_type = to_backend_type(ctx.for_making_type(), ir_type)?;
                Some(LLVMBuildIntToPtr(
                    builder.get(),
                    value,
                    backend_type,
                    c"".as_ptr(),
                ))
            }
            Instr::PointerToInteger(value, ir_type) => {
                let value = build_value(ctx, value_catalog, builder, value)?;
                let backend_type = to_backend_type(ctx.for_making_type(), ir_type)?;
                Some(LLVMBuildPtrToInt(
                    builder.get(),
                    value,
                    backend_type,
                    c"".as_ptr(),
                ))
            }
            Instr::FloatToInteger(value, ir_type, sign) => {
                let value = build_value(ctx, value_catalog, builder, value)?;
                let backend_type = to_backend_type(ctx.for_making_type(), ir_type)?;

                Some(match sign {
                    IntegerSign::Signed => {
                        LLVMBuildFPToSI(builder.get(), value, backend_type, c"".as_ptr())
                    }
                    IntegerSign::Unsigned => {
                        LLVMBuildFPToUI(builder.get(), value, backend_type, c"".as_ptr())
                    }
                })
            }
            Instr::IntegerToFloat(value, ir_type, sign) => {
                let value = build_value(ctx, value_catalog, builder, value)?;
                let backend_type = to_backend_type(ctx.for_making_type(), ir_type)?;

                Some(match sign {
                    IntegerSign::Signed => {
                        LLVMBuildSIToFP(builder.get(), value, backend_type, c"".as_ptr())
                    }
                    IntegerSign::Unsigned => {
                        LLVMBuildUIToFP(builder.get(), value, backend_type, c"".as_ptr())
                    }
                })
            }
        };

        value_catalog.push(ir_basicblock_id, result);
    }

    Ok(())
}

unsafe fn build_binary_operands<'env>(
    ctx: &BackendCtx<'_, 'env>,
    value_catalog: &ValueCatalog,
    builder: &Builder<'env>,
    operands: &ir::BinaryOperands<'env>,
) -> Result<(LLVMValueRef, LLVMValueRef), ErrorDiagnostic> {
    let left = build_value(ctx, value_catalog, builder, &operands.left)?;
    let right = build_value(ctx, value_catalog, builder, &operands.right)?;
    Ok((left, right))
}

unsafe fn promote_variadic_argument(
    builder: &Builder,
    target: &Target,
    value: LLVMValueRef,
) -> LLVMValueRef {
    let llvm_type = LLVMTypeOf(value);

    if llvm_type.is_float() {
        LLVMBuildFPExt(builder.get(), value, LLVMDoubleType(), c"".as_ptr())
    } else if llvm_type.is_integer() {
        let c_int_size = BitUnits::from(target.int_layout().width);

        if llvm_type.integer_width() < c_int_size {
            builder.zext(value, LLVMTypeRef::new_int(c_int_size))
        } else {
            value
        }
    } else {
        value
    }
}

fn promote_variadic_argument_type<'env>(
    ctx: &BackendCtx<'_, 'env>,
    builder: &Builder<'env>,
    target: &Target,
    ir_type: &'env ir::Type<'env>,
) -> &'env ir::Type<'env> {
    assert_eq!(
        target.int_layout().width,
        ByteUnits::of(4),
        "we're assuming sizeof C int is 32 bits for now"
    );

    if is_promotable_integer_type_for_abi(ir_type) {
        return ctx
            .alloc
            .alloc(ir::Type::I(IntegerBits::Bits32, IntegerSign::Signed));
    }

    match ir_type {
        // Promote floats to 64-bit
        ir::Type::F(FloatSize::Bits32) => ctx.alloc.alloc(ir::Type::F(FloatSize::Bits64)),

        // Promote atomics based on what's inside
        ir::Type::Atomic(inner_type) => {
            ctx.alloc
                .alloc(ir::Type::Atomic(promote_variadic_argument_type(
                    ctx, builder, target, inner_type,
                )))
        }

        // Otherwise, keep the same type
        _ => ir_type,
    }
}

unsafe fn emit_call<'env>(
    ctx: &BackendCtx<'_, 'env>,
    builder: &Builder<'env>,
    call: &Call<'env>,
    fn_ctx: &FnCtx,
    value_catalog: &mut ValueCatalog,
) -> Result<LLVMValueRef, ErrorDiagnostic> {
    let ir_function = &ctx.ir_module.funcs[call.func];

    let skeleton = ctx
        .func_skeletons
        .get(&call.func)
        .expect("ir function to exist");

    // Keep track of maximum vector width passed/recieved/returned within current function
    // SAFETY: This is okay, as we promise to never read from this
    // without synchronizing
    fn_ctx
        .max_vector_width_bytes
        .max(skeleton.max_vector_width, atomic::Ordering::Relaxed);

    let function_value = skeleton.function;

    let mut args = call
        .args
        .iter()
        .enumerate()
        .map(|(i, argument)| {
            build_value(ctx, value_catalog, builder, argument).map(|value| {
                if i >= ir_function.params.len() {
                    promote_variadic_argument(builder, &ctx.meta.target, value)
                } else {
                    value
                }
            })
        })
        .collect::<Result<Vec<LLVMValueRef>, _>>()?;

    if ir_function.abide_abi {
        let abi_function = skeleton.abi_function.as_ref().expect("abi function");

        let variadic_argument_types =
            call.unpromoted_variadic_arg_types
                .iter()
                .map(|argument_type| {
                    promote_variadic_argument_type(ctx, builder, &ctx.meta.target, argument_type)
                });

        let argument_types_iter = ir_function.params.iter().chain(variadic_argument_types);

        let num_required = ir_function.params.len();

        // If we're using variadic arguments, then we have to re-generate the ABI
        // function signature for the way we're calling it
        let abi_function_approximation = (abi_function.parameter_types.len() < args.len())
            .then(|| {
                ABIFunction::new(
                    ctx,
                    argument_types_iter.clone(),
                    num_required,
                    &ir_function.return_type,
                    ir_function.is_cstyle_variadic,
                )
                .map(Cow::Owned)
            })
            .transpose()?
            .unwrap_or(Cow::Borrowed(abi_function));

        // After generating the function signature, we should have ABI parameter information for each argument
        assert_eq!(abi_function_approximation.parameter_types.len(), args.len());

        // NOTE: We shouldn't need inalloca, since we intend to target
        // only x86_64 Windows GNU on Windows, as opposed to older MSVC ABIs.
        // This may change in the future
        assert!(
            skeleton
                .abi_function
                .as_ref()
                .and_then(|abi_function| abi_function.inalloca_combined_struct.as_ref())
                .is_none()
        );

        let params_mapping = ParamsMapping::new(
            ctx,
            &ctx.type_layout_cache,
            &abi_function_approximation,
            ctx.ir_module,
        );

        let mut ir_call_args = vec![null_mut(); params_mapping.llvm_arity()].into_boxed_slice();

        // We can optionally choose to override the return destination if desired
        let return_destination: Option<Address> = None;

        let abi_return_info = &abi_function_approximation.return_type;
        let ir_return_type = &ir_function.return_type;

        let sret_pointer = match abi_return_info.abi_type.kind {
            ABITypeKind::Indirect(_)
            | ABITypeKind::CoerceAndExpand(_)
            | ABITypeKind::InAlloca(_) => {
                let sret_pointer = if let Some(return_destination) = return_destination.as_ref() {
                    Cow::Borrowed(return_destination)
                } else {
                    Cow::Owned(
                        build_mem_tmp(
                            ctx,
                            builder,
                            fn_ctx.alloca_point.expect("function has body"),
                            ir_return_type,
                            c"tmp",
                        )?
                        .into(),
                    )
                };

                if let Some(sret_index) = params_mapping.sret_index() {
                    ir_call_args[sret_index] = sret_pointer.base_pointer();
                } else {
                    assert!(
                        !abi_return_info.abi_type.kind.is_in_alloca(),
                        "we don't support inalloca here yet"
                    );
                }

                Some(sret_pointer)
            }
            _ => None,
        };

        // NOTE: For initial simplicity, we will always save the stack pointer
        // and then restore if after performing a C-ABI compliant function call.
        // In some cases this isn't necessary

        let alloca_point = fn_ctx.alloca_point.expect("has function body");
        let saved_stack_pointer = builder.save_stack_pointer(ctx);

        for (argument, argument_type, abi_param, param_mapping) in izip!(
            args.iter().copied(),
            argument_types_iter,
            abi_function_approximation.parameter_types.iter(),
            params_mapping.params().iter(),
        ) {
            if let Some((padding_index, padding_type)) = param_mapping
                .padding_index()
                .zip(abi_param.abi_type.padding_type().flatten())
            {
                ir_call_args[padding_index] = unsafe { LLVMGetUndef(padding_type) };
            }

            match &abi_param.abi_type.kind {
                ABITypeKind::Direct(_) | ABITypeKind::Extend(_) => direct_or_extend(
                    builder,
                    ctx,
                    alloca_point,
                    argument,
                    argument_type,
                    abi_param,
                    param_mapping,
                    &skeleton.function_type,
                    &mut ir_call_args[..],
                )?,
                ABITypeKind::Indirect(_) | ABITypeKind::IndirectAliased(_) => indirect(
                    ctx,
                    builder,
                    alloca_point,
                    argument,
                    argument_type,
                    abi_param,
                    param_mapping,
                    &mut ir_call_args[..],
                )?,
                ABITypeKind::Ignore => assert_eq!(param_mapping.range().len(), 0),
                ABITypeKind::Expand(_) => expand(
                    builder,
                    ctx,
                    alloca_point,
                    argument,
                    argument_type,
                    param_mapping,
                    &skeleton.function_type,
                    &mut ir_call_args,
                )?,
                ABITypeKind::CoerceAndExpand(CoerceAndExpand { alignment, .. }) => {
                    coerce_and_expand(
                        builder,
                        ctx,
                        alloca_point,
                        argument,
                        argument_type,
                        abi_param,
                        param_mapping,
                        *alignment,
                        &mut ir_call_args[..],
                    )?
                }
                ABITypeKind::InAlloca(_) => {
                    unimplemented!("inalloca pass mode not supported at the moment")
                }
            }
        }

        // NOTE: We don't support inalloca here yet
        assert!(
            abi_function_approximation
                .inalloca_combined_struct
                .is_none()
        );

        let num_required = ir_function.params.len();

        let actual_abi_function = ir_function
            .is_cstyle_variadic
            .then(|| {
                ABIFunction::new(
                    ctx,
                    ir_function.params.iter(),
                    num_required,
                    &ir_function.return_type,
                    ir_function.is_cstyle_variadic,
                )
            })
            .transpose()?
            .map(Cow::Owned::<ABIFunction>)
            .unwrap_or_else(|| Cow::Borrowed(&abi_function_approximation));

        let actual_params_mapping = ir_function
            .is_cstyle_variadic
            .then(|| {
                Cow::Owned(ParamsMapping::new(
                    ctx,
                    &ctx.type_layout_cache,
                    &actual_abi_function,
                    ctx.ir_module,
                ))
            })
            .unwrap_or_else(|| Cow::Borrowed(&params_mapping));

        let function_type = get_abi_function_type(
            ctx,
            ir_function,
            &actual_abi_function,
            &actual_params_mapping,
        )?;

        let returned = LLVMBuildCall2(
            builder.get(),
            function_type,
            function_value,
            ir_call_args.as_mut_ptr(),
            ir_call_args.len().try_into().unwrap(),
            c"".as_ptr(),
        );

        builder.restore_stack_pointer(ctx, saved_stack_pointer);

        let abi_type = &abi_return_info.abi_type;
        let return_type = &ir_function.return_type;

        let return_value = match &abi_type.kind {
            ABITypeKind::Direct(_) | ABITypeKind::Extend(_) => {
                let backend_return_type =
                    unsafe { to_backend_type(ctx.for_making_type(), return_type)? };
                let coerce_to_type = abi_type.coerce_to_type().flatten().unwrap();
                let direct_offset = abi_type.get_direct_offset().unwrap();

                if coerce_to_type == backend_return_type && direct_offset.is_zero() {
                    if has_scalar_evaluation_kind(return_type) {
                        if unsafe { LLVMTypeOf(returned) } != backend_return_type {
                            builder.bitcast(returned, backend_return_type)
                        } else {
                            returned
                        }
                    } else if return_type.is_complex() {
                        todo!("complex types not supported via direct/extend ABI return mode yet")
                    } else {
                        returned
                    }
                } else {
                    let tmp: Cow<Address> = if let Some(destination) = return_destination.as_ref() {
                        Cow::Borrowed(destination)
                    } else {
                        Cow::Owned(Address::from(build_mem_tmp(
                            ctx,
                            builder,
                            alloca_point,
                            return_type,
                            c"coerce",
                        )?))
                    };

                    let address = emit_address_at_offset(builder, ctx.target_data, abi_type, &tmp);
                    build_coerced_store(builder, ctx.target_data, returned, &address, alloca_point);
                    convert_tmp_to_rvalue(builder, &address, return_type)
                }
            }
            ABITypeKind::Indirect(_) | ABITypeKind::InAlloca(_) => convert_tmp_to_rvalue(
                builder,
                sret_pointer.as_ref().expect("sret pointer"),
                &ir_function.return_type,
            ),
            ABITypeKind::Ignore => unsafe {
                LLVMGetUndef(to_backend_type(
                    ctx.for_making_type(),
                    &ir_function.return_type,
                )?)
            },
            ABITypeKind::Expand(_) | ABITypeKind::IndirectAliased(_) => {
                panic!("invalid return value ABI")
            }
            ABITypeKind::CoerceAndExpand(coerce_and_expand) => {
                let coerce_to_type = coerce_and_expand.coerce_to_type;
                let address = sret_pointer.expect("sret pointer");
                let returned_type = unsafe { LLVMTypeOf(returned) };

                assert_eq!(
                    returned_type,
                    coerce_and_expand.unpadded_coerce_and_expand_type
                );

                let element_types = coerce_to_type.field_types();
                let requires_extract = returned_type.is_struct();
                let mut unpadded_field_index = 0;

                for (element_index, element_type) in element_types.iter().copied().enumerate() {
                    if is_padding_for_coerce_expand(element_type) {
                        continue;
                    }

                    let element_address = builder.gep_struct(
                        ctx.target_data,
                        &address,
                        element_index,
                        Some(element_types.as_slice()),
                    );

                    let element = if requires_extract {
                        let value = unsafe {
                            LLVMBuildExtractValue(
                                builder.get(),
                                returned,
                                unpadded_field_index,
                                c"".as_ptr(),
                            )
                        };
                        unpadded_field_index += 1;
                        value
                    } else {
                        assert_eq!(unpadded_field_index, 0);
                        returned
                    };

                    builder.store(element, &element_address);
                }

                convert_tmp_to_rvalue(builder, &address, &ir_function.return_type)
            }
        };

        Ok(return_value)
    } else {
        let function_type = get_unabi_function_type(ctx.for_making_type(), ir_function)?;

        Ok(LLVMBuildCall2(
            builder.get(),
            function_type,
            function_value,
            args.as_mut_ptr(),
            args.len().try_into().unwrap(),
            c"".as_ptr(),
        ))
    }
}

fn convert_tmp_to_rvalue(builder: &Builder, address: &Address, ir_type: &ir::Type) -> LLVMValueRef {
    if has_scalar_evaluation_kind(ir_type) {
        emit_load_of_scalar(builder, address, Volatility::Normal, ir_type)
    } else if ir_type.is_complex() {
        todo!("convert_tmp_to_rvalue not supported for complex types yet")
    } else {
        builder.load(address, Volatility::Normal)
    }
}

#[allow(clippy::too_many_arguments)]
fn indirect<'env>(
    ctx: &BackendCtx<'_, 'env>,
    builder: &Builder<'env>,
    alloca_point: LLVMValueRef,
    argument: LLVMValueRef,
    argument_type: &'env ir::Type<'env>,
    abi_param: &ABIParam<'env>,
    param_mapping: &Param,
    ir_call_args: &mut [LLVMValueRef],
) -> Result<(), ErrorDiagnostic> {
    let param_range = param_mapping.range();
    assert_eq!(param_range.len(), 1);

    // NOTE: We shouldn't have to copy the value into an aligned temporary in all cases.
    // TODO: Skip this when possible

    let abi_argument = Address::from(build_mem_tmp_without_cast(
        ctx,
        builder,
        alloca_point,
        argument_type,
        abi_param.abi_type.indirect_align().unwrap(),
        c"byvaltmp",
    )?);

    builder.store(argument, &abi_argument);

    ir_call_args[param_range.start] = abi_argument.base_pointer();
    Ok(())
}

fn coerce_and_expand<'env>(
    builder: &Builder<'env>,
    ctx: &BackendCtx<'_, 'env>,
    alloca_point: LLVMValueRef,
    argument: LLVMValueRef,
    argument_type: &'env ir::Type<'env>,
    abi_param: &ABIParam<'env>,
    param_mapping: &Param,
    alignment: ByteUnits,
    ir_call_args: &mut [LLVMValueRef],
) -> Result<(), ErrorDiagnostic> {
    let coerce_type = abi_param.abi_type.coerce_and_expand_type().unwrap();
    let backend_argument_type = unsafe { to_backend_type(ctx.for_making_type(), argument_type)? };

    // TODO: Is this alignment proper?
    // We'll max these just to be safe for now, but it should only depend on
    // the alignment of the coerce aggregate and of the original type.
    let abi_alignment = ctx.target_data.abi_size_of_type(backend_argument_type);
    let layout_alignment = ctx.type_layout_cache.get(argument_type).alignment;
    let alignment = alignment.max(abi_alignment).max(layout_alignment);

    // TODO: We shouldn't need to do this in most cases.
    let address = Address::from(build_tmp_alloca_address(
        builder,
        alloca_point,
        backend_argument_type,
        alignment,
        c"coerceandexpand.tmp",
        None,
    ));
    builder.store(argument, &address);

    assert!(coerce_type.is_struct());
    let address = address.with_element_type(coerce_type);
    let field_types = coerce_type.field_types();

    for (field_i, (llvm_arg_i, element_type)) in param_mapping
        .range()
        .iter()
        .zip(field_types.iter().copied())
        .enumerate()
    {
        if is_padding_for_coerce_expand(element_type) {
            continue;
        }

        let element_address = builder.gep_struct(
            ctx.target_data,
            &address,
            field_i,
            Some(field_types.as_slice()),
        );

        ir_call_args[llvm_arg_i] = builder.load(&element_address, Volatility::Normal);
    }

    Ok(())
}

fn expand<'env>(
    builder: &Builder<'env>,
    ctx: &BackendCtx<'_, 'env>,
    alloca_point: LLVMValueRef,
    argument: LLVMValueRef,
    argument_type: &'env ir::Type<'env>,
    param_mapping: &Param,
    function_type: &FunctionType,
    ir_call_args: &mut [LLVMValueRef],
) -> Result<(), ErrorDiagnostic> {
    let mut llvm_arg_i_iterator = param_mapping.range().iter();

    // TODO: We shouldn't need to do this in most cases.
    let backend_argument_type = unsafe { to_backend_type(ctx.for_making_type(), argument_type)? };
    let alignment = ctx.type_layout_cache.get(argument_type).alignment;
    let argument_address = Address::from(build_tmp_alloca_address(
        builder,
        alloca_point,
        backend_argument_type,
        alignment,
        c"coerceandexpand.tmp",
        None,
    ));
    builder.store(argument, &argument_address);

    expand_type_to_args(
        builder,
        ctx,
        &argument_address,
        argument_type,
        function_type,
        &mut llvm_arg_i_iterator,
        ir_call_args,
    )?;
    assert!(llvm_arg_i_iterator.next().is_none());

    Ok(())
}

fn expand_type_to_args<'env>(
    builder: &Builder<'env>,
    ctx: &BackendCtx<'_, 'env>,
    argument_address: &Address,
    argument_type: &'env ir::Type<'env>,
    function_type: &FunctionType,
    llvm_arg_i_iterator: &mut impl Iterator<Item = usize>,
    ir_call_args: &mut [LLVMValueRef],
) -> Result<(), ErrorDiagnostic> {
    let expansion = get_type_expansion(ctx, argument_type, &ctx.type_layout_cache, ctx.ir_module);

    match expansion {
        TypeExpansion::FixedArray(fixed_array) => {
            for index in 0..fixed_array.length {
                let element_address = builder.gep(ctx.target_data, argument_address, 0, index);
                let element_type = &fixed_array.inner;

                expand_type_to_args(
                    builder,
                    ctx,
                    &element_address,
                    element_type,
                    function_type,
                    llvm_arg_i_iterator,
                    ir_call_args,
                )?
            }
        }
        TypeExpansion::Record(fields) => {
            let precomputed_field_types = fields
                .iter()
                .map(|field| unsafe { to_backend_type(ctx.for_making_type(), &field.ir_type) })
                .collect::<Result<Box<[_]>, _>>()?;

            for (field_i, field) in fields.iter().enumerate() {
                let element_address = builder.gep_struct(
                    ctx.target_data,
                    argument_address,
                    field_i,
                    Some(&precomputed_field_types),
                );
                let element_type = &field.ir_type;

                expand_type_to_args(
                    builder,
                    ctx,
                    &element_address,
                    element_type,
                    function_type,
                    llvm_arg_i_iterator,
                    ir_call_args,
                )?
            }
        }
        TypeExpansion::Complex(_) => {
            todo!("expand_type_to_args not supported for complex types yet")
        }
        TypeExpansion::None => {
            let argument =
                emit_load_of_scalar(builder, argument_address, Volatility::Normal, argument_type);

            let llvm_arg_i = llvm_arg_i_iterator
                .next()
                .expect("argument position to insert into");

            let argument =
                if unsafe { LLVMTypeOf(argument) } == function_type.parameters[llvm_arg_i] {
                    argument
                } else {
                    builder.bitcast(argument, function_type.parameters[llvm_arg_i])
                };

            ir_call_args[llvm_arg_i] = argument;
        }
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn direct_or_extend<'env>(
    builder: &Builder<'env>,
    ctx: &BackendCtx<'_, 'env>,
    alloca_point: LLVMValueRef,
    argument: LLVMValueRef,
    argument_type: &'env ir::Type<'env>,
    abi_param: &ABIParam<'env>,
    param_mapping: &Param,
    llvm_function_type: &FunctionType,
    ir_call_args: &mut [LLVMValueRef],
) -> Result<(), ErrorDiagnostic> {
    let param_range = param_mapping.range();
    let coerce_to_type = abi_param.abi_type.coerce_to_type().flatten().unwrap();
    let direct_offset = abi_param.abi_type.get_direct_offset().unwrap();

    if !coerce_to_type.is_struct() {
        let backend_argument_type =
            unsafe { to_backend_type(ctx.for_making_type(), argument_type)? };

        if coerce_to_type == backend_argument_type && direct_offset.is_zero() {
            return trivial_direct_or_extend(
                builder,
                argument,
                abi_param,
                param_mapping,
                llvm_function_type,
                ir_call_args,
            );
        }
    }

    // Coerce by memory
    let source = Address::from(build_mem_tmp(
        ctx,
        builder,
        alloca_point,
        argument_type,
        c"coerce",
    )?);
    builder.store(argument, &source);

    let source = emit_address_at_offset(builder, ctx.target_data, &abi_param.abi_type, &source);

    if coerce_to_type.is_struct()
        && abi_param.abi_type.is_direct()
        && abi_param.abi_type.can_be_flattened().unwrap()
    {
        let source_type = source.element_type();
        let source_type_size = ctx.target_data.abi_size_of_type(source_type);
        let destination_type_size = ctx.target_data.abi_size_of_type(coerce_to_type);

        let source = if source_type_size < destination_type_size {
            let tmp_alloca = Address::from(build_tmp_alloca_address(
                builder,
                alloca_point,
                source_type,
                source.base.alignment,
                c"upscale",
                None,
            ));

            let num_bytes_literal = unsafe {
                LLVMConstInt(
                    LLVMInt64Type(),
                    source_type_size.bytes().try_into().unwrap(),
                    false as _,
                )
            };

            builder.memcpy(&tmp_alloca, &source, num_bytes_literal);
            tmp_alloca
        } else {
            source.with_element_type(coerce_to_type)
        };

        let precomputed_field_types = coerce_to_type.field_types();

        assert_eq!(param_range.len(), precomputed_field_types.len());

        for field_i in 0..precomputed_field_types.len() {
            let element_pointer = builder.gep_struct(
                ctx.target_data,
                &source,
                field_i,
                Some(precomputed_field_types.as_slice()),
            );

            ir_call_args[param_range.start + field_i] =
                builder.load(&element_pointer, Volatility::Normal);
        }
        return Ok(());
    }

    assert_eq!(param_range.len(), 1);

    ir_call_args[param_range.start] =
        build_coerced_load(ctx, builder, &source, coerce_to_type, alloca_point);

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn trivial_direct_or_extend(
    builder: &Builder,
    argument: LLVMValueRef,
    abi_param: &ABIParam,
    param_mapping: &Param,
    llvm_function_type: &FunctionType,
    ir_call_args: &mut [LLVMValueRef],
) -> Result<(), ErrorDiagnostic> {
    let param_range = param_mapping.range();
    assert_eq!(param_range.len(), 1);

    let mut argument = argument;
    let mut llvm_argument_type = unsafe { LLVMTypeOf(argument) };
    let coerce_to_type = abi_param.abi_type.coerce_to_type().flatten().unwrap();

    if coerce_to_type != llvm_argument_type && llvm_argument_type.is_integer() {
        argument = builder.zext_with_name(argument, coerce_to_type, c"up");
        llvm_argument_type = unsafe { LLVMTypeOf(argument) };
    }

    if let Some(expected_llvm_argument_type) = llvm_function_type.parameters.get(param_range.start)
    {
        if llvm_argument_type != *expected_llvm_argument_type {
            argument = builder.bitcast(argument, *expected_llvm_argument_type);
        }
    }

    ir_call_args[param_range.start] = argument;
    Ok(())
}
