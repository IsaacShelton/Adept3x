mod builder;
mod ctx;
mod intrinsics;
mod module;
mod null_terminated_string;
mod target_data;
mod target_machine;
mod value_catalog;
mod variable_stack;

use self::{
    builder::Builder, ctx::BackendContext, module::BackendModule,
    null_terminated_string::build_literal_cstring, target_data::TargetData,
    target_machine::TargetMachine, value_catalog::ValueCatalog,
};
use crate::{
    error::CompilerError,
    ir::{self, Instruction},
};
use colored::Colorize;
use cstr::cstr;
use ir::IntegerSign;
use llvm_sys::{
    analysis::{LLVMVerifierFailureAction::LLVMPrintMessageAction, LLVMVerifyModule},
    core::{
        LLVMAddFunction, LLVMAddGlobal, LLVMAppendBasicBlock, LLVMBuildAdd, LLVMBuildAlloca,
        LLVMBuildCall2, LLVMBuildCondBr, LLVMBuildExtractValue, LLVMBuildGEP2, LLVMBuildICmp,
        LLVMBuildInsertValue, LLVMBuildLoad2, LLVMBuildMul, LLVMBuildRet, LLVMBuildSDiv,
        LLVMBuildSRem, LLVMBuildStore, LLVMBuildSub, LLVMBuildUDiv, LLVMBuildURem,
        LLVMBuildUnreachable, LLVMConstInt, LLVMConstReal, LLVMDisposeMessage, LLVMDoubleType,
        LLVMFloatType, LLVMFunctionType, LLVMGetParam, LLVMGetUndef, LLVMInt16Type, LLVMInt1Type,
        LLVMInt32Type, LLVMInt64Type, LLVMInt8Type, LLVMPointerType, LLVMPositionBuilderAtEnd,
        LLVMPrintModuleToString, LLVMSetExternallyInitialized, LLVMSetFunctionCallConv,
        LLVMSetLinkage, LLVMSetThreadLocal, LLVMStructType, LLVMVoidType,
    },
    prelude::{LLVMBasicBlockRef, LLVMBool, LLVMTypeRef, LLVMValueRef},
    target::{
        LLVMSetModuleDataLayout, LLVM_InitializeAllAsmParsers, LLVM_InitializeAllAsmPrinters,
        LLVM_InitializeAllTargetInfos, LLVM_InitializeAllTargetMCs, LLVM_InitializeAllTargets,
    },
    target_machine::{
        LLVMCodeGenFileType, LLVMCodeGenOptLevel, LLVMCodeModel, LLVMGetDefaultTargetTriple,
        LLVMGetTargetFromTriple, LLVMRelocMode, LLVMTargetRef,
    },
    LLVMCallConv, LLVMLinkage,
};
use llvm_sys::{
    core::{
        LLVMBuildBitCast, LLVMBuildFPExt, LLVMBuildFPTrunc, LLVMBuildSExt, LLVMBuildTrunc,
        LLVMBuildZExt,
    },
    LLVMIntPredicate::*,
};
use std::{
    cell::OnceCell,
    ffi::{c_char, c_double, c_ulonglong, CStr, CString, OsStr},
    mem::MaybeUninit,
    path::Path,
    process::Command,
    ptr::null_mut,
};

pub unsafe fn llvm_backend(
    ir_module: &ir::Module,
    output_object_filepath: &Path,
    output_binary_filepath: &Path,
) -> Result<(), CompilerError> {
    LLVM_InitializeAllTargetInfos();
    LLVM_InitializeAllTargets();
    LLVM_InitializeAllTargetMCs();
    LLVM_InitializeAllAsmParsers();
    LLVM_InitializeAllAsmPrinters();

    let module_name = CString::new(output_object_filepath.to_str().expect("valid utf8")).unwrap();
    let triple = get_triple();
    let target = get_target_from_triple(&triple)?;
    let cpu = CString::new("generic").unwrap();
    let features = CString::new("").unwrap();
    let level = LLVMCodeGenOptLevel::LLVMCodeGenLevelDefault;
    let use_pic = false;
    let reloc = if use_pic {
        LLVMRelocMode::LLVMRelocPIC
    } else {
        LLVMRelocMode::LLVMRelocDefault
    };
    let code_model = LLVMCodeModel::LLVMCodeModelDefault;

    let backend_module = BackendModule::new(&module_name);
    let target_machine =
        TargetMachine::new(target, &triple, &cpu, &features, level, reloc, code_model);
    let target_data = TargetData::new(&target_machine);
    LLVMSetModuleDataLayout(backend_module.get(), target_data.get());

    let mut ctx = BackendContext::new(ir_module, &backend_module, &target_data);

    create_structures(&mut ctx);
    create_static_variables()?;
    create_globals(&mut ctx)?;
    create_function_heads(&mut ctx)?;
    create_function_bodies(&mut ctx)?;
    implement_static_init()?;
    implement_static_deinit()?;

    let mut llvm_emit_error_message: *mut c_char = null_mut();

    #[allow(unused_variables)]
    let module_representation = CStr::from_ptr(LLVMPrintModuleToString(backend_module.get()));

    // println!("{}", module_representation.to_string_lossy());

    let output_object_filename =
        CString::new(output_object_filepath.to_str().expect("valid utf8")).unwrap();

    if LLVMVerifyModule(backend_module.get(), LLVMPrintMessageAction, null_mut()) == 1 {
        println!(
            "{}",
            "\n---- WARNING: llvm module verification failed! ----".yellow()
        );
    }

    llvm_sys::target_machine::LLVMTargetMachineEmitToFile(
        target_machine.get(),
        backend_module.get(),
        output_object_filename.into_raw(),
        LLVMCodeGenFileType::LLVMObjectFile,
        &mut llvm_emit_error_message,
    );

    if !llvm_emit_error_message.is_null() {
        return Err(CompilerError::during_backend(
            CString::from_raw(llvm_emit_error_message)
                .to_string_lossy()
                .into_owned(),
        ));
    }

    // Link resulting object file to create executable
    Command::new("gcc")
        .args([
            output_object_filepath.as_os_str(),
            OsStr::new("-o"),
            output_binary_filepath.as_os_str(),
        ])
        .spawn()
        .expect("Failed to link");
    Ok(())
}

unsafe fn create_structures(ctx: &mut BackendContext) {
    for (structure_key, ir_structure) in ctx.ir_module.structures.iter() {
        let mut subtypes = to_backend_types(ctx, &ir_structure.fields);

        let struct_type = LLVMStructType(
            subtypes.as_mut_ptr(),
            subtypes.len().try_into().unwrap(),
            ir_structure.is_packed.into(),
        );

        ctx.structure_cache
            .insert(structure_key.clone(), struct_type);
    }
}

unsafe fn create_static_variables() -> Result<(), CompilerError> {
    Ok(())
}

unsafe fn create_globals(ctx: &mut BackendContext) -> Result<(), CompilerError> {
    for (global_ref, global) in ctx.ir_module.globals.iter() {
        let backend_type = to_backend_type(ctx, &global.ir_type);

        let name = CString::new(global.mangled_name.as_bytes()).unwrap();
        let backend_global = LLVMAddGlobal(ctx.backend_module.get(), backend_type, name.as_ptr());

        LLVMSetLinkage(
            backend_global,
            if global.is_foreign {
                LLVMLinkage::LLVMExternalLinkage
            } else {
                LLVMLinkage::LLVMInternalLinkage
            },
        );

        if global.is_thread_local {
            LLVMSetThreadLocal(backend_global, true.into());
        }

        if !global.is_foreign {
            // In order to prevent aggressive optimizations from removing necessary internal global
            // variables, we'll mark them as externally-initialized
            LLVMSetExternallyInitialized(backend_global, true.into());
        }

        ctx.globals.insert(global_ref.clone(), backend_global);
    }

    Ok(())
}

unsafe fn create_function_heads(ctx: &mut BackendContext) -> Result<(), CompilerError> {
    for (function_ref, function) in ctx.ir_module.functions.iter() {
        let mut parameters = to_backend_types(ctx, &function.parameters);
        let return_type = to_backend_type(ctx, &function.return_type);

        let name = CString::new(function.mangled_name.as_bytes()).unwrap();

        let function_type = LLVMFunctionType(
            return_type,
            parameters.as_mut_ptr(),
            parameters.len() as u32,
            function.is_cstyle_variadic as i32,
        );

        let skeleton = LLVMAddFunction(ctx.backend_module.get(), name.as_ptr(), function_type);
        LLVMSetFunctionCallConv(skeleton, LLVMCallConv::LLVMCCallConv as u32);

        if !function.is_foreign && !function.is_exposed {
            LLVMSetLinkage(skeleton, LLVMLinkage::LLVMPrivateLinkage);
        }

        ctx.func_skeletons.insert(function_ref.clone(), skeleton);
    }

    Ok(())
}

unsafe fn create_function_bodies(ctx: &mut BackendContext) -> Result<(), CompilerError> {
    for (ir_function_ref, skeleton) in ctx.func_skeletons.iter() {
        if let Some(ir_function) = ctx.ir_module.functions.get(ir_function_ref) {
            let builder = Builder::new();
            let mut value_catalog = ValueCatalog::new(ir_function.basicblocks.len());

            let basicblocks =
                ir_function
                    .basicblocks
                    .iter()
                    .enumerate()
                    .map(|(id, ir_basicblock)| {
                        (
                            id,
                            ir_basicblock,
                            LLVMAppendBasicBlock(*skeleton, cstr!("").as_ptr()),
                        )
                    });

            let overflow_basicblock: OnceCell<LLVMBasicBlockRef> = OnceCell::new();

            for (ir_basicblock_id, ir_basicblock, llvm_basicblock) in basicblocks {
                create_function_block(
                    ctx,
                    &mut value_catalog,
                    &overflow_basicblock,
                    &builder,
                    ir_basicblock_id,
                    ir_basicblock,
                    llvm_basicblock,
                    *skeleton,
                );
            }
        }
    }

    Ok(())
}

unsafe fn create_function_block(
    ctx: &BackendContext,
    value_catalog: &mut ValueCatalog,
    overflow_basicblock: &OnceCell<LLVMBasicBlockRef>,
    builder: &Builder,
    ir_basicblock_id: usize,
    ir_basicblock: &ir::BasicBlock,
    mut llvm_basicblock: LLVMBasicBlockRef,
    function_skeleton: LLVMValueRef,
) {
    LLVMPositionBuilderAtEnd(builder.get(), llvm_basicblock);

    for instruction in ir_basicblock.iter() {
        let result = match instruction {
            Instruction::Return(value) => {
                let _ = LLVMBuildRet(
                    builder.get(),
                    value.as_ref().map_or_else(
                        || null_mut(),
                        |value| build_value(ctx.backend_module, value_catalog, &builder, value),
                    ),
                );
                None
            }
            Instruction::Alloca(ir_type) => Some(LLVMBuildAlloca(
                builder.get(),
                to_backend_type(ctx, ir_type),
                cstr!("").as_ptr(),
            )),
            Instruction::Parameter(index) => Some(LLVMGetParam(function_skeleton, *index)),
            Instruction::GlobalVariable(global_ref) => Some(
                *ctx.globals
                    .get(global_ref)
                    .expect("referenced global to exist"),
            ),
            Instruction::Store(store) => {
                let source = build_value(ctx.backend_module, value_catalog, builder, &store.source);
                let destination = build_value(
                    ctx.backend_module,
                    value_catalog,
                    builder,
                    &store.destination,
                );
                let _ = LLVMBuildStore(builder.get(), source, destination);
                None
            }
            Instruction::Load((value, ir_type)) => {
                let pointer = build_value(ctx.backend_module, value_catalog, builder, value);
                let llvm_type = to_backend_type(ctx, ir_type);
                Some(LLVMBuildLoad2(
                    builder.get(),
                    llvm_type,
                    pointer,
                    cstr!("").as_ptr(),
                ))
            }
            Instruction::Call(call) => {
                let function_type = get_function_type(
                    ctx,
                    ctx.ir_module
                        .functions
                        .get(&call.function)
                        .expect("callee to exist"),
                );

                let function_value = *ctx
                    .func_skeletons
                    .get(&call.function)
                    .expect("ir function to exist");

                let mut arguments = call
                    .arguments
                    .iter()
                    .map(|argument| {
                        build_value(ctx.backend_module, value_catalog, builder, argument)
                    })
                    .collect::<Vec<_>>();

                Some(LLVMBuildCall2(
                    builder.get(),
                    function_type,
                    function_value,
                    arguments.as_mut_ptr(),
                    arguments.len().try_into().unwrap(),
                    cstr!("").as_ptr(),
                ))
            }
            Instruction::Add(operands) => {
                let (left, right) =
                    build_binary_operands(ctx.backend_module, value_catalog, builder, operands);

                Some(LLVMBuildAdd(builder.get(), left, right, cstr!("").as_ptr()))
            }
            Instruction::Checked(operation, operands) => {
                let (left, right) =
                    build_binary_operands(ctx.backend_module, value_catalog, builder, operands);

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
            Instruction::Subtract(operands) => {
                let (left, right) =
                    build_binary_operands(ctx.backend_module, value_catalog, builder, operands);

                Some(LLVMBuildSub(builder.get(), left, right, cstr!("").as_ptr()))
            }
            Instruction::Multiply(operands) => {
                let (left, right) =
                    build_binary_operands(ctx.backend_module, value_catalog, builder, operands);

                Some(LLVMBuildMul(builder.get(), left, right, cstr!("").as_ptr()))
            }
            Instruction::Divide(operands, sign) => {
                let (left, right) =
                    build_binary_operands(ctx.backend_module, value_catalog, builder, operands);

                Some(match sign {
                    IntegerSign::Signed => {
                        LLVMBuildSDiv(builder.get(), left, right, cstr!("").as_ptr())
                    }
                    IntegerSign::Unsigned => {
                        LLVMBuildUDiv(builder.get(), left, right, cstr!("").as_ptr())
                    }
                })
            }
            Instruction::Modulus(operands, sign) => {
                let (left, right) =
                    build_binary_operands(ctx.backend_module, value_catalog, builder, operands);

                Some(match sign {
                    IntegerSign::Signed => {
                        LLVMBuildSRem(builder.get(), left, right, cstr!("").as_ptr())
                    }
                    IntegerSign::Unsigned => {
                        LLVMBuildURem(builder.get(), left, right, cstr!("").as_ptr())
                    }
                })
            }
            Instruction::Equals(operands) => {
                let (left, right) =
                    build_binary_operands(ctx.backend_module, value_catalog, builder, operands);

                Some(LLVMBuildICmp(
                    builder.get(),
                    LLVMIntEQ,
                    left,
                    right,
                    cstr!("").as_ptr(),
                ))
            }
            Instruction::NotEquals(operands) => {
                let (left, right) =
                    build_binary_operands(ctx.backend_module, value_catalog, builder, operands);

                Some(LLVMBuildICmp(
                    builder.get(),
                    LLVMIntNE,
                    left,
                    right,
                    cstr!("").as_ptr(),
                ))
            }
            Instruction::LessThan(operands, sign) => {
                let (left, right) =
                    build_binary_operands(ctx.backend_module, value_catalog, builder, operands);

                Some(LLVMBuildICmp(
                    builder.get(),
                    match sign {
                        IntegerSign::Signed => LLVMIntSLT,
                        IntegerSign::Unsigned => LLVMIntULT,
                    },
                    left,
                    right,
                    cstr!("").as_ptr(),
                ))
            }
            Instruction::LessThanEq(operands, sign) => {
                let (left, right) =
                    build_binary_operands(ctx.backend_module, value_catalog, builder, operands);

                Some(LLVMBuildICmp(
                    builder.get(),
                    match sign {
                        IntegerSign::Signed => LLVMIntSLE,
                        IntegerSign::Unsigned => LLVMIntULE,
                    },
                    left,
                    right,
                    cstr!("").as_ptr(),
                ))
            }
            Instruction::GreaterThan(operands, sign) => {
                let (left, right) =
                    build_binary_operands(ctx.backend_module, value_catalog, builder, operands);

                Some(LLVMBuildICmp(
                    builder.get(),
                    match sign {
                        IntegerSign::Signed => LLVMIntSGT,
                        IntegerSign::Unsigned => LLVMIntUGT,
                    },
                    left,
                    right,
                    cstr!("").as_ptr(),
                ))
            }
            Instruction::GreaterThanEq(operands, sign) => {
                let (left, right) =
                    build_binary_operands(ctx.backend_module, value_catalog, builder, operands);

                Some(LLVMBuildICmp(
                    builder.get(),
                    match sign {
                        IntegerSign::Signed => LLVMIntSGE,
                        IntegerSign::Unsigned => LLVMIntUGE,
                    },
                    left,
                    right,
                    cstr!("").as_ptr(),
                ))
            }
            Instruction::Bitcast(value, ir_type) => {
                let value = build_value(ctx.backend_module, value_catalog, builder, value);
                let backend_type = to_backend_type(ctx, ir_type);
                Some(LLVMBuildBitCast(
                    builder.get(),
                    value,
                    backend_type,
                    cstr!("").as_ptr(),
                ))
            }
            Instruction::ZeroExtend(value, ir_type) => {
                let value = build_value(ctx.backend_module, value_catalog, builder, value);
                let backend_type = to_backend_type(ctx, ir_type);
                Some(LLVMBuildZExt(
                    builder.get(),
                    value,
                    backend_type,
                    cstr!("").as_ptr(),
                ))
            }
            Instruction::SignExtend(value, ir_type) => {
                let value = build_value(ctx.backend_module, value_catalog, builder, value);
                let backend_type = to_backend_type(ctx, ir_type);
                Some(LLVMBuildSExt(
                    builder.get(),
                    value,
                    backend_type,
                    cstr!("").as_ptr(),
                ))
            }
            Instruction::FloatExtend(value, ir_type) => {
                let value = build_value(ctx.backend_module, value_catalog, builder, value);
                let backend_type = to_backend_type(ctx, ir_type);
                Some(LLVMBuildFPExt(
                    builder.get(),
                    value,
                    backend_type,
                    cstr!("").as_ptr(),
                ))
            }
            Instruction::Truncate(value, ir_type) => {
                let value = build_value(ctx.backend_module, value_catalog, builder, value);
                let backend_type = to_backend_type(ctx, ir_type);
                Some(LLVMBuildTrunc(
                    builder.get(),
                    value,
                    backend_type,
                    cstr!("").as_ptr(),
                ))
            }
            Instruction::TruncateFloat(value, ir_type) => {
                let value = build_value(ctx.backend_module, value_catalog, builder, value);
                let backend_type = to_backend_type(ctx, ir_type);
                Some(LLVMBuildFPTrunc(
                    builder.get(),
                    value,
                    backend_type,
                    cstr!("").as_ptr(),
                ))
            }
            Instruction::Member(pointer_value, structure_ref, index) => {
                let pointer =
                    build_value(ctx.backend_module, value_catalog, builder, pointer_value);
                let backend_type = *ctx
                    .structure_cache
                    .get(structure_ref)
                    .expect("referenced structure to exist");

                let mut indices = [
                    LLVMConstInt(LLVMInt32Type(), 0, true.into()),
                    LLVMConstInt(LLVMInt32Type(), (*index).try_into().unwrap(), true.into()),
                ];

                Some(LLVMBuildGEP2(
                    builder.get(),
                    backend_type,
                    pointer,
                    indices.as_mut_ptr(),
                    indices.len().try_into().unwrap(),
                    cstr!("").as_ptr(),
                ))
            }
            Instruction::StructureLiteral(ir_type, values) => {
                let backend_type = to_backend_type(ctx, ir_type);
                let mut literal = LLVMGetUndef(backend_type);

                for (index, value) in values.iter().enumerate() {
                    let backend_value =
                        build_value(ctx.backend_module, value_catalog, builder, value);

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
        };

        value_catalog.push(ir_basicblock_id, result);
    }
}

unsafe fn get_function_type(ctx: &BackendContext, function: &ir::Function) -> LLVMTypeRef {
    let return_type = to_backend_type(ctx, &function.return_type);
    let mut parameters = to_backend_types(ctx, &function.parameters);
    let is_vararg = if function.is_cstyle_variadic { 1 } else { 0 };

    LLVMFunctionType(
        return_type,
        parameters.as_mut_ptr(),
        parameters.len().try_into().unwrap(),
        is_vararg,
    )
}

unsafe fn build_binary_operands(
    backend_module: &BackendModule,
    value_catalog: &ValueCatalog,
    builder: &Builder,
    operands: &ir::BinaryOperands,
) -> (LLVMValueRef, LLVMValueRef) {
    let left = build_value(backend_module, value_catalog, builder, &operands.left);
    let right = build_value(backend_module, value_catalog, builder, &operands.right);
    (left, right)
}

unsafe fn build_value(
    backend_module: &BackendModule,
    value_catalog: &ValueCatalog,
    _builder: &Builder,
    value: &ir::Value,
) -> LLVMValueRef {
    match value {
        ir::Value::Literal(literal) => match literal {
            ir::Literal::Boolean(value) => {
                LLVMConstInt(LLVMInt1Type(), *value as c_ulonglong, false as LLVMBool)
            }
            ir::Literal::Signed8(value) => {
                LLVMConstInt(LLVMInt8Type(), *value as c_ulonglong, true as LLVMBool)
            }
            ir::Literal::Signed16(value) => {
                LLVMConstInt(LLVMInt16Type(), *value as c_ulonglong, true as LLVMBool)
            }
            ir::Literal::Signed32(value) => {
                LLVMConstInt(LLVMInt32Type(), *value as c_ulonglong, true as LLVMBool)
            }
            ir::Literal::Signed64(value) => {
                LLVMConstInt(LLVMInt64Type(), *value as c_ulonglong, true as LLVMBool)
            }
            ir::Literal::Unsigned8(value) => {
                LLVMConstInt(LLVMInt8Type(), *value as c_ulonglong, false as LLVMBool)
            }
            ir::Literal::Unsigned16(value) => {
                LLVMConstInt(LLVMInt16Type(), *value as c_ulonglong, false as LLVMBool)
            }
            ir::Literal::Unsigned32(value) => {
                LLVMConstInt(LLVMInt32Type(), *value as c_ulonglong, false as LLVMBool)
            }
            ir::Literal::Unsigned64(value) => {
                LLVMConstInt(LLVMInt64Type(), *value as c_ulonglong, false as LLVMBool)
            }
            ir::Literal::Float32(value) => LLVMConstReal(LLVMFloatType(), *value as c_double),
            ir::Literal::Float64(value) => LLVMConstReal(LLVMDoubleType(), *value as c_double),
            ir::Literal::NullTerminatedString(value) => {
                build_literal_cstring(backend_module.get(), value)
            }
        },
        ir::Value::Reference(reference) => value_catalog
            .get(reference)
            .expect("referenced value exists"),
    }
}

unsafe fn to_backend_type(ctx: &BackendContext, ir_type: &ir::Type) -> LLVMTypeRef {
    match ir_type {
        ir::Type::Void => LLVMVoidType(),
        ir::Type::Boolean => LLVMInt1Type(),
        ir::Type::S8 | ir::Type::U8 => LLVMInt8Type(),
        ir::Type::S16 | ir::Type::U16 => LLVMInt16Type(),
        ir::Type::S32 | ir::Type::U32 => LLVMInt32Type(),
        ir::Type::S64 | ir::Type::U64 => LLVMInt64Type(),
        ir::Type::F32 => LLVMFloatType(),
        ir::Type::F64 => LLVMDoubleType(),
        ir::Type::Pointer(to) => LLVMPointerType(to_backend_type(ctx, to), 0),
        ir::Type::UntypedEnum(_) => panic!("Cannot convert untyped enum to backend type"),
        ir::Type::AnonymousComposite(composite) => {
            let mut subtypes = to_backend_types(ctx, &composite.subtypes);

            LLVMStructType(
                subtypes.as_mut_ptr(),
                subtypes.len() as u32,
                composite.is_packed.into(),
            )
        }
        ir::Type::Structure(structure_ref) => ctx
            .structure_cache
            .get(structure_ref)
            .expect("referenced structure to exist")
            .clone(),
        ir::Type::Function(function) => {
            let return_type = to_backend_type(ctx, &function.return_type);
            let mut params = to_backend_types(ctx, &function.parameters);

            LLVMPointerType(
                LLVMFunctionType(
                    return_type,
                    params.as_mut_ptr(),
                    params.len() as u32,
                    function.is_cstyle_variadic as i32,
                ),
                0,
            )
        }
    }
}

unsafe fn to_backend_types(ctx: &BackendContext, ir_types: &[ir::Type]) -> Vec<LLVMTypeRef> {
    ir_types.iter().map(|ty| to_backend_type(ctx, ty)).collect()
}

unsafe fn implement_static_init() -> Result<(), CompilerError> {
    Ok(())
}

unsafe fn implement_static_deinit() -> Result<(), CompilerError> {
    Ok(())
}

unsafe fn get_triple() -> CString {
    return CString::from_raw(LLVMGetDefaultTargetTriple());
}

unsafe fn get_target_from_triple(triple: &CStr) -> Result<LLVMTargetRef, CompilerError> {
    let mut target: MaybeUninit<LLVMTargetRef> = MaybeUninit::zeroed();
    let mut error_message: MaybeUninit<*mut i8> = MaybeUninit::zeroed();

    if LLVMGetTargetFromTriple(
        triple.as_ptr(),
        target.as_mut_ptr(),
        error_message.as_mut_ptr(),
    ) != 0
    {
        let message = CStr::from_ptr(error_message.assume_init()).to_owned();
        LLVMDisposeMessage(error_message.assume_init());
        Err(CompilerError::during_backend(
            message
                .into_string()
                .unwrap_or_else(|_| "Failed to get target triple for platform".into()),
        ))
    } else {
        Ok(target.assume_init())
    }
}
