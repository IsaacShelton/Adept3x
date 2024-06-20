mod abi;
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
    builder::{Builder, PhiRelocation},
    ctx::BackendContext,
    module::BackendModule,
    null_terminated_string::build_literal_cstring,
    target_data::TargetData,
    target_machine::TargetMachine,
    value_catalog::ValueCatalog,
};
use crate::{
    ir::{self, Instruction},
    resolved::{FloatOrInteger, StructureRef},
    show::Show,
    source_file_cache::SourceFileCache,
};
use colored::Colorize;
use cstr::cstr;
use ir::{FloatOrSign, IntegerSign};
use llvm_sys::{
    analysis::{LLVMVerifierFailureAction::LLVMPrintMessageAction, LLVMVerifyModule},
    core::{
        LLVMAddFunction, LLVMAddGlobal, LLVMAddIncoming, LLVMAppendBasicBlock, LLVMArrayType2, LLVMBuildAShr, LLVMBuildAdd, LLVMBuildAlloca, LLVMBuildAnd, LLVMBuildArrayMalloc, LLVMBuildBr, LLVMBuildCall2, LLVMBuildCondBr, LLVMBuildExtractValue, LLVMBuildFAdd, LLVMBuildFCmp, LLVMBuildFDiv, LLVMBuildFMul, LLVMBuildFNeg, LLVMBuildFRem, LLVMBuildFSub, LLVMBuildFree, LLVMBuildGEP2, LLVMBuildICmp, LLVMBuildInsertValue, LLVMBuildIsNotNull, LLVMBuildIsNull, LLVMBuildLShr, LLVMBuildLoad2, LLVMBuildMalloc, LLVMBuildMul, LLVMBuildNeg, LLVMBuildNot, LLVMBuildOr, LLVMBuildPhi, LLVMBuildRet, LLVMBuildSDiv, LLVMBuildSRem, LLVMBuildShl, LLVMBuildStore, LLVMBuildSub, LLVMBuildUDiv, LLVMBuildURem, LLVMBuildUnreachable, LLVMBuildXor, LLVMConstInt, LLVMConstNull, LLVMConstReal, LLVMDisposeMessage, LLVMDoubleType, LLVMFloatType, LLVMFunctionType, LLVMGetIntTypeWidth, LLVMGetParam, LLVMGetUndef, LLVMInt16Type, LLVMInt1Type, LLVMInt32Type, LLVMInt64Type, LLVMInt8Type, LLVMPointerType, LLVMPositionBuilderAtEnd, LLVMSetExternallyInitialized, LLVMSetFunctionCallConv, LLVMSetInitializer, LLVMSetLinkage, LLVMSetThreadLocal, LLVMStructType, LLVMVoidType
    },
    prelude::{LLVMBasicBlockRef, LLVMBool, LLVMTypeRef, LLVMValueRef},
    target::{
        LLVMABISizeOfType, LLVMSetModuleDataLayout, LLVM_InitializeAllAsmParsers,
        LLVM_InitializeAllAsmPrinters, LLVM_InitializeAllTargetInfos, LLVM_InitializeAllTargetMCs,
        LLVM_InitializeAllTargets,
    },
    target_machine::{
        LLVMCodeGenFileType, LLVMCodeGenOptLevel, LLVMCodeModel, LLVMGetDefaultTargetTriple,
        LLVMGetTargetFromTriple, LLVMRelocMode, LLVMTargetRef,
    },
    LLVMCallConv, LLVMLinkage,
    LLVMRealPredicate::*,
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
    collections::HashSet,
    ffi::{c_char, c_double, c_ulonglong, CStr, CString, OsStr},
    mem::MaybeUninit,
    path::Path,
    process::Command,
    ptr::null_mut,
};

#[derive(Clone, Debug)]
pub struct BackendError {
    pub message: String,
}

impl From<String> for BackendError {
    fn from(message: String) -> Self {
        Self { message }
    }
}

impl From<&str> for BackendError {
    fn from(message: &str) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl Show for BackendError {
    fn show(
        &self,
        w: &mut impl std::fmt::Write,
        _source_file_cache: &SourceFileCache,
    ) -> std::fmt::Result {
        write!(w, "error: {}", self.message)
    }
}

pub unsafe fn llvm_backend(
    ir_module: &ir::Module,
    output_object_filepath: &Path,
    output_binary_filepath: &Path,
) -> Result<(), BackendError> {
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

    create_static_variables()?;
    create_globals(&mut ctx)?;
    create_function_heads(&mut ctx)?;
    create_function_bodies(&mut ctx)?;
    implement_static_init()?;
    implement_static_deinit()?;

    let mut llvm_emit_error_message: *mut c_char = null_mut();

    // Print generated LLVM IR?
    {
        // println!("{}", CStr::from_ptr(LLVMPrintModuleToString(backend_module.get())).to_string_lossy());
    }

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
        return Err(CString::from_raw(llvm_emit_error_message)
            .to_string_lossy()
            .into_owned()
            .into());
    }

    // Link resulting object file to create executable
    let mut command = Command::new("gcc")
        .args([
            output_object_filepath.as_os_str(),
            OsStr::new("-o"),
            output_binary_filepath.as_os_str(),
        ])
        .spawn()
        .expect("Failed to link");

    match command.wait() {
        Ok(status) => {
            if !status.success() {
                return Err(BackendError {
                    message: "Failed to link".into(),
                });
            }
        }
        Err(_) => {
            return Err(BackendError {
                message: "Failed to spawn linker".into(),
            });
        }
    }

    Ok(())
}

unsafe fn create_structure(
    ctx: &BackendContext,
    structure_key: &StructureRef,
    visited: &mut HashSet<StructureRef>,
) -> Result<LLVMTypeRef, BackendError> {
    // Ensure not infinite size
    if visited.contains(structure_key) {
        // TODO: Improve error message
        return Err(BackendError {
            message: "Recursive data structure".into(),
        });
    }

    // Get cached type or insert computed type
    ctx.structure_cache.try_insert_cloned(*structure_key, |_| {
        let ir_structure = ctx
            .ir_module
            .structures
            .get(structure_key)
            .expect("referenced IR structure to exist");

        visited.insert(*structure_key);
        let mut subtypes = to_backend_types(ctx, &ir_structure.fields, visited)?;
        visited.remove(structure_key);

        Ok(LLVMStructType(
            subtypes.as_mut_ptr(),
            subtypes.len().try_into().unwrap(),
            ir_structure.is_packed.into(),
        ))
    })
}

unsafe fn create_static_variables() -> Result<(), BackendError> {
    Ok(())
}

unsafe fn create_globals(ctx: &mut BackendContext) -> Result<(), BackendError> {
    for (global_ref, global) in ctx.ir_module.globals.iter() {
        let backend_type = to_backend_type(ctx, &global.ir_type, &mut HashSet::default())?;

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
            LLVMSetInitializer(backend_global, LLVMGetUndef(backend_type));
        }

        ctx.globals.insert(global_ref.clone(), backend_global);
    }

    Ok(())
}

unsafe fn create_function_heads(ctx: &mut BackendContext) -> Result<(), BackendError> {
    for (function_ref, function) in ctx.ir_module.functions.iter() {
        let mut parameters = to_backend_types(ctx, &function.parameters, &mut HashSet::default())?;
        let return_type = to_backend_type(ctx, &function.return_type, &mut HashSet::default())?;

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

unsafe fn create_function_bodies(ctx: &mut BackendContext) -> Result<(), BackendError> {
    for (ir_function_ref, skeleton) in ctx.func_skeletons.iter() {
        if let Some(ir_function) = ctx.ir_module.functions.get(ir_function_ref) {
            let mut builder = Builder::new();
            let mut value_catalog = ValueCatalog::new(ir_function.basicblocks.len());

            let basicblocks = ir_function
                .basicblocks
                .iter()
                .enumerate()
                .map(|(id, ir_basicblock)| {
                    (
                        id,
                        ir_basicblock,
                        LLVMAppendBasicBlock(*skeleton, cstr!("").as_ptr()),
                    )
                })
                .collect::<Vec<_>>();

            let overflow_basicblock: OnceCell<LLVMBasicBlockRef> = OnceCell::new();

            for (ir_basicblock_id, ir_basicblock, llvm_basicblock) in basicblocks.iter() {
                create_function_block(
                    ctx,
                    &mut value_catalog,
                    &overflow_basicblock,
                    &builder,
                    *ir_basicblock_id,
                    ir_basicblock,
                    *llvm_basicblock,
                    *skeleton,
                    &basicblocks,
                )?;
            }

            for relocation in builder.take_phi_relocations().iter() {
                for incoming in relocation.incoming.iter() {
                    let (_, _, backend_block) = basicblocks
                        .get(incoming.basicblock_id)
                        .expect("backend basicblock referenced by phi node to exist");

                    let mut backend_block = *backend_block;

                    let mut value = build_value(ctx, &value_catalog, &builder, &incoming.value)?;

                    LLVMAddIncoming(relocation.phi_node, &mut value, &mut backend_block, 1);
                }
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
    basicblocks: &[(usize, &ir::BasicBlock, LLVMBasicBlockRef)],
) -> Result<(), BackendError> {
    LLVMPositionBuilderAtEnd(builder.get(), llvm_basicblock);

    // Set used while traversing generated types, reusable
    let mut visited = HashSet::default();

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
                to_backend_type(ctx, ir_type, &mut visited)?,
                cstr!("").as_ptr(),
            )),
            Instruction::Malloc(ir_type) => {
                let backend_type = to_backend_type(ctx, ir_type, &mut visited)?;
                Some(LLVMBuildMalloc(
                    builder.get(),
                    backend_type,
                    cstr!("").as_ptr(),
                ))
            }
            Instruction::MallocArray(ir_type, count) => {
                let backend_type = to_backend_type(ctx, ir_type, &mut visited)?;
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
                let backend_type = to_backend_type(ctx, ir_type, &mut visited)?;
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
                let llvm_type = to_backend_type(ctx, ir_type, &mut visited)?;
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
                let backend_type = to_backend_type(ctx, ir_type, &mut visited)?;
                Some(LLVMBuildBitCast(
                    builder.get(),
                    value,
                    backend_type,
                    cstr!("").as_ptr(),
                ))
            }
            Instruction::ZeroExtend(value, ir_type) => {
                let value = build_value(ctx, value_catalog, builder, value)?;
                let backend_type = to_backend_type(ctx, ir_type, &mut visited)?;
                Some(LLVMBuildZExt(
                    builder.get(),
                    value,
                    backend_type,
                    cstr!("").as_ptr(),
                ))
            }
            Instruction::SignExtend(value, ir_type) => {
                let value = build_value(ctx, value_catalog, builder, value)?;
                let backend_type = to_backend_type(ctx, ir_type, &mut visited)?;
                Some(LLVMBuildSExt(
                    builder.get(),
                    value,
                    backend_type,
                    cstr!("").as_ptr(),
                ))
            }
            Instruction::FloatExtend(value, ir_type) => {
                let value = build_value(ctx, value_catalog, builder, value)?;
                let backend_type = to_backend_type(ctx, ir_type, &mut visited)?;
                Some(LLVMBuildFPExt(
                    builder.get(),
                    value,
                    backend_type,
                    cstr!("").as_ptr(),
                ))
            }
            Instruction::Truncate(value, ir_type) => {
                let value = build_value(ctx, value_catalog, builder, value)?;
                let backend_type = to_backend_type(ctx, ir_type, &mut visited)?;
                Some(LLVMBuildTrunc(
                    builder.get(),
                    value,
                    backend_type,
                    cstr!("").as_ptr(),
                ))
            }
            Instruction::TruncateFloat(value, ir_type) => {
                let value = build_value(ctx, value_catalog, builder, value)?;
                let backend_type = to_backend_type(ctx, ir_type, &mut visited)?;
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
                        to_backend_type(ctx, ir_struct_type, &mut visited)?
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

                let backend_item_type = to_backend_type(ctx, ir_item_type, &mut visited)?;
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
                let backend_type = to_backend_type(ctx, ir_type, &mut visited)?;
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
                let backend_type = to_backend_type(ctx, &phi.ir_type, &mut visited)?;
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

unsafe fn get_function_type(
    ctx: &BackendContext,
    function: &ir::Function,
) -> Result<LLVMTypeRef, BackendError> {
    let mut visited = HashSet::default();
    let return_type = to_backend_type(ctx, &function.return_type, &mut visited)?;
    let mut parameters = to_backend_types(ctx, &function.parameters, &mut visited)?;
    let is_vararg = if function.is_cstyle_variadic { 1 } else { 0 };

    Ok(LLVMFunctionType(
        return_type,
        parameters.as_mut_ptr(),
        parameters.len().try_into().unwrap(),
        is_vararg,
    ))
}

unsafe fn build_binary_operands(
    ctx: &BackendContext<'_>,
    value_catalog: &ValueCatalog,
    builder: &Builder,
    operands: &ir::BinaryOperands,
) -> Result<(LLVMValueRef, LLVMValueRef), BackendError> {
    let left = build_value(ctx, value_catalog, builder, &operands.left)?;
    let right = build_value(ctx, value_catalog, builder, &operands.right)?;
    Ok((left, right))
}

unsafe fn build_value(
    ctx: &BackendContext<'_>,
    value_catalog: &ValueCatalog,
    _builder: &Builder,
    value: &ir::Value,
) -> Result<LLVMValueRef, BackendError> {
    Ok(match value {
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
                build_literal_cstring(ctx.backend_module.get(), value)
            }
            ir::Literal::Void => LLVMGetUndef(LLVMVoidType()),
            ir::Literal::Zeroed(ir_type) => {
                let backend_type = to_backend_type(ctx, ir_type, &mut HashSet::new())?;
                LLVMConstNull(backend_type)
            }
        },
        ir::Value::Reference(reference) => value_catalog
            .get(reference)
            .expect("referenced value exists"),
    })
}

unsafe fn to_backend_type(
    ctx: &BackendContext,
    ir_type: &ir::Type,
    visited: &mut HashSet<StructureRef>,
) -> Result<LLVMTypeRef, BackendError> {
    Ok(match ir_type {
        ir::Type::Void => LLVMVoidType(),
        ir::Type::Boolean => LLVMInt1Type(),
        ir::Type::S8 | ir::Type::U8 => LLVMInt8Type(),
        ir::Type::S16 | ir::Type::U16 => LLVMInt16Type(),
        ir::Type::S32 | ir::Type::U32 => LLVMInt32Type(),
        ir::Type::S64 | ir::Type::U64 => LLVMInt64Type(),
        ir::Type::F32 => LLVMFloatType(),
        ir::Type::F64 => LLVMDoubleType(),
        ir::Type::Pointer(to) => LLVMPointerType(to_backend_type(ctx, to, visited)?, 0),
        ir::Type::UntypedEnum(_) => panic!("Cannot convert untyped enum to backend type"),
        ir::Type::AnonymousComposite(composite) => {
            let mut subtypes = to_backend_types(ctx, &composite.subtypes, visited)?;

            LLVMStructType(
                subtypes.as_mut_ptr(),
                subtypes.len() as u32,
                composite.is_packed.into(),
            )
        }
        ir::Type::Structure(structure_ref) => create_structure(ctx, structure_ref, visited)?,
        ir::Type::Function(function) => {
            let return_type = to_backend_type(ctx, &function.return_type, visited)?;
            let mut params = to_backend_types(ctx, &function.parameters, visited)?;

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
        ir::Type::FixedArray(fixed_array) => {
            let element_type = to_backend_type(ctx, &fixed_array.inner, visited)?;
            LLVMArrayType2(element_type, fixed_array.size)
        }
    })
}

unsafe fn to_backend_types(
    ctx: &BackendContext,
    ir_types: &[ir::Type],
    visited: &mut HashSet<StructureRef>,
) -> Result<Vec<LLVMTypeRef>, BackendError> {
    let mut results = Vec::with_capacity(ir_types.len());

    for ir_type in ir_types.iter() {
        results.push(to_backend_type(ctx, ir_type, visited)?);
    }

    Ok(results)
}

unsafe fn implement_static_init() -> Result<(), BackendError> {
    Ok(())
}

unsafe fn implement_static_deinit() -> Result<(), BackendError> {
    Ok(())
}

unsafe fn get_triple() -> CString {
    return CString::from_raw(LLVMGetDefaultTargetTriple());
}

unsafe fn get_target_from_triple(triple: &CStr) -> Result<LLVMTargetRef, BackendError> {
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
        Err(message
            .into_string()
            .unwrap_or_else(|_| "Failed to get target triple for platform".into())
            .into())
    } else {
        Ok(target.assume_init())
    }
}
