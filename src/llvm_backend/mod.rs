mod builder;
mod ctx;
mod intrinsics;
mod module;
mod target_data;
mod target_machine;
mod value_catalog;
mod variable_stack;

use self::{
    builder::Builder, ctx::BackendContext, module::BackendModule, target_data::TargetData,
    target_machine::TargetMachine,
};
use crate::{
    error::CompilerError,
    ir::{self, Instruction},
};
use colored::Colorize;
use cstr::cstr;
use llvm_sys::{
    analysis::{LLVMVerifierFailureAction::LLVMPrintMessageAction, LLVMVerifyModule},
    core::{
        LLVMAddAttributeAtIndex, LLVMAddFunction, LLVMAddGlobal, LLVMAppendBasicBlock,
        LLVMArrayType2, LLVMBuildCall2, LLVMBuildRet, LLVMConstGEP2, LLVMConstInt, LLVMConstReal,
        LLVMConstString, LLVMCreateEnumAttribute, LLVMDisposeMessage, LLVMDisposeModule,
        LLVMDoubleType, LLVMFloatType, LLVMFunctionType, LLVMGetEnumAttributeKindForName,
        LLVMGetGlobalContext, LLVMInt16Type, LLVMInt1Type, LLVMInt32Type, LLVMInt64Type,
        LLVMInt8Type, LLVMModuleCreateWithName, LLVMPointerType, LLVMPositionBuilderAtEnd,
        LLVMPrintModuleToString, LLVMSetFunctionCallConv, LLVMSetGlobalConstant,
        LLVMSetInitializer, LLVMSetLinkage, LLVMStructType, LLVMVoidType,
    },
    prelude::{LLVMBasicBlockRef, LLVMBool, LLVMModuleRef, LLVMTypeRef, LLVMValueRef},
    target::{
        LLVMSetModuleDataLayout, LLVM_InitializeAllAsmParsers, LLVM_InitializeAllAsmPrinters,
        LLVM_InitializeAllTargetInfos, LLVM_InitializeAllTargetMCs, LLVM_InitializeAllTargets,
    },
    target_machine::{
        LLVMCodeGenFileType, LLVMCodeGenOptLevel, LLVMCodeModel, LLVMCreateTargetDataLayout,
        LLVMCreateTargetMachine, LLVMDisposeTargetMachine, LLVMGetDefaultTargetTriple,
        LLVMGetTargetFromTriple, LLVMGetTargetName, LLVMRelocMode, LLVMTarget,
        LLVMTargetMachineRef, LLVMTargetRef,
    },
    LLVMAttributeFunctionIndex, LLVMCallConv, LLVMLinkage, LLVMModule,
};
use slotmap::SlotMap;
use std::{
    error::Error,
    ffi::{c_char, c_double, c_ulonglong, CStr, CString},
    fmt::Display,
    mem::MaybeUninit,
    process::Command,
    ptr::{null, null_mut},
};

pub unsafe fn llvm_backend(ir_module: &ir::Module) -> Result<(), CompilerError> {
    LLVM_InitializeAllTargetInfos();
    LLVM_InitializeAllTargets();
    LLVM_InitializeAllTargetMCs();
    LLVM_InitializeAllAsmParsers();
    LLVM_InitializeAllAsmPrinters();

    let module_name = CString::new("a.o").unwrap();
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
    create_globals()?;
    create_function_heads(&mut ctx)?;
    create_function_bodies(&mut ctx)?;
    implement_static_init()?;
    implement_static_deinit()?;

    let mut llvm_emit_error_message: *mut c_char = null_mut();

    let module_representation = CStr::from_ptr(LLVMPrintModuleToString(backend_module.get()));
    println!("{}", module_representation.to_string_lossy());

    let mut output_object_filename = CString::new("a.o").unwrap();

    if (LLVMVerifyModule(backend_module.get(), LLVMPrintMessageAction, null_mut()) == 1) {
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
        .args(["a.o"])
        .spawn()
        .expect("Failed to link");
    Ok(())
}

unsafe fn create_static_variables() -> Result<(), CompilerError> {
    Ok(())
}

unsafe fn create_globals() -> Result<(), CompilerError> {
    Ok(())
}

unsafe fn create_function_heads(ctx: &mut BackendContext) -> Result<(), CompilerError> {
    for (function_ref, function) in ctx.ir_module.functions.iter() {
        let mut parameters: Vec<LLVMTypeRef> =
            to_backend_types(ctx.backend_module, &function.parameters);
        let return_type = to_backend_type(ctx.backend_module, &function.return_type);

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

        ctx.func_skeletons.insert(function_ref, skeleton);
    }

    Ok(())
}

unsafe fn create_function_bodies(ctx: &mut BackendContext) -> Result<(), CompilerError> {
    for (ir_function_ref, skeleton) in ctx.func_skeletons.iter() {
        if let Some(ir_function) = ctx.ir_module.functions.get(*ir_function_ref) {
            let builder = Builder::new();

            let basicblocks = ir_function
                .basicblocks
                .iter()
                .map(|ir_basicblock| {
                    (
                        ir_basicblock,
                        LLVMAppendBasicBlock(*skeleton, cstr!("").as_ptr()),
                    )
                })
                .collect::<Vec<_>>();

            for (ir_basicblock, llvm_basicblock) in basicblocks.iter() {
                create_function_block(ctx, &builder, ir_basicblock, *llvm_basicblock);
            }
        }
    }

    Ok(())
}

unsafe fn create_function_block(
    ctx: &BackendContext,
    builder: &Builder,
    ir_basicblock: &ir::BasicBlock,
    llvm_basicblock: LLVMBasicBlockRef,
) {
    LLVMPositionBuilderAtEnd(builder.get(), llvm_basicblock);

    for instruction in ir_basicblock.iter() {
        match instruction {
            Instruction::Return(value) => {
                let _ = LLVMBuildRet(
                    builder.get(),
                    value.as_ref().map_or_else(
                        || null_mut(),
                        |value| build_value(ctx.backend_module, &builder, value),
                    ),
                );
            }
            Instruction::Call(call) => {
                let function_type = get_function_type(
                    ctx.backend_module,
                    ctx.ir_module
                        .functions
                        .get(call.function)
                        .expect("callee to exist"),
                );

                let function_value = *ctx
                    .func_skeletons
                    .get(&call.function)
                    .expect("ir function to exist");

                let mut arguments = call
                    .arguments
                    .iter()
                    .map(|argument| build_value(ctx.backend_module, builder, argument))
                    .collect::<Vec<_>>();

                let _ = LLVMBuildCall2(
                    builder.get(),
                    function_type,
                    function_value,
                    arguments.as_mut_ptr(),
                    arguments.len().try_into().unwrap(),
                    cstr!("").as_ptr(),
                );
            }
        }
    }
}

unsafe fn get_function_type(
    backend_module: &BackendModule,
    function: &ir::Function,
) -> LLVMTypeRef {
    let return_type = to_backend_type(backend_module, &function.return_type);
    let mut parameters = to_backend_types(backend_module, &function.parameters);
    let is_vararg = if function.is_cstyle_variadic { 1 } else { 0 };

    LLVMFunctionType(
        return_type,
        parameters.as_mut_ptr(),
        parameters.len().try_into().unwrap(),
        is_vararg,
    )
}

unsafe fn build_value(
    backend_module: &BackendModule,
    builder: &Builder,
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
                let length = value.as_bytes_with_nul().len();
                let storage_type = LLVMArrayType2(LLVMInt8Type(), length.try_into().unwrap());

                let read_only =
                    LLVMAddGlobal(backend_module.get(), storage_type, cstr!("").as_ptr());
                LLVMSetLinkage(read_only, LLVMLinkage::LLVMInternalLinkage);
                LLVMSetGlobalConstant(read_only, true as i32);
                LLVMSetInitializer(
                    read_only,
                    LLVMConstString(value.as_ptr(), length.try_into().unwrap(), true as i32),
                );

                let mut indicies = [
                    LLVMConstInt(LLVMInt32Type(), 0, true as i32),
                    LLVMConstInt(LLVMInt32Type(), 0, true as i32),
                ];

                LLVMConstGEP2(
                    storage_type,
                    read_only,
                    indicies.as_mut_ptr(),
                    indicies.len() as _,
                )
            }
        },
        ir::Value::Reference(_) => todo!(),
    }
}

unsafe fn to_backend_type(backend_module: &BackendModule, ir_type: &ir::Type) -> LLVMTypeRef {
    match ir_type {
        ir::Type::Void => LLVMVoidType(),
        ir::Type::Boolean => LLVMInt1Type(),
        ir::Type::S8 | ir::Type::U8 => LLVMInt8Type(),
        ir::Type::S16 | ir::Type::U16 => LLVMInt16Type(),
        ir::Type::S32 | ir::Type::U32 => LLVMInt32Type(),
        ir::Type::S64 | ir::Type::U64 => LLVMInt64Type(),
        ir::Type::F32 => LLVMFloatType(),
        ir::Type::F64 => LLVMDoubleType(),
        ir::Type::Pointer(to) => LLVMPointerType(to_backend_type(backend_module, to), 0),
        ir::Type::UntypedEnum(_) => panic!("Cannot convert untyped enum to backend type"),
        ir::Type::Composite(composite) => {
            let mut subtypes = to_backend_types(backend_module, &composite.subtypes);

            LLVMStructType(
                subtypes.as_mut_ptr(),
                subtypes.len() as u32,
                composite.is_packed.into(),
            )
        }
        ir::Type::Function(function) => {
            let return_type = to_backend_type(backend_module, &function.return_type);
            let mut params = to_backend_types(backend_module, &function.parameters);

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

unsafe fn to_backend_types(
    backend_module: &BackendModule,
    ir_types: &[ir::Type],
) -> Vec<LLVMTypeRef> {
    ir_types
        .iter()
        .map(|ty| to_backend_type(backend_module, ty))
        .collect()
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
