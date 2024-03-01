mod builder;
mod ctx;
mod intrinsics;
mod module;
mod target_data;
mod target_machine;
mod value_catalog;
mod variable_stack;

use crate::{
    error::CompilerError,
    ir::{self},
};
use colored::Colorize;
use cstr::cstr;
use llvm_sys::{
    core::{
        LLVMAddAttributeAtIndex, LLVMAddFunction, LLVMCreateEnumAttribute, LLVMDisposeMessage,
        LLVMDisposeModule, LLVMDoubleType, LLVMFloatType, LLVMFunctionType,
        LLVMGetEnumAttributeKindForName, LLVMGetGlobalContext, LLVMInt16Type, LLVMInt1Type,
        LLVMInt32Type, LLVMInt64Type, LLVMInt8Type, LLVMModuleCreateWithName, LLVMPointerType,
        LLVMPrintModuleToString, LLVMSetFunctionCallConv, LLVMSetLinkage, LLVMStructType,
        LLVMVoidType,
    },
    prelude::{LLVMModuleRef, LLVMTypeRef},
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
    ffi::{c_char, CStr, CString},
    fmt::Display,
    mem::MaybeUninit,
    ptr::null_mut,
};

use self::{
    ctx::BackendContext, module::BackendModule, target_data::TargetData,
    target_machine::TargetMachine,
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

    llvm_sys::target_machine::LLVMTargetMachineEmitToFile(
        target_machine.get(),
        backend_module.get(),
        output_object_filename.into_raw(),
        LLVMCodeGenFileType::LLVMObjectFile,
        &mut llvm_emit_error_message,
    );

    if !llvm_emit_error_message.is_null() {
        Err(CompilerError::during_backend(
            CString::from_raw(llvm_emit_error_message)
                .to_string_lossy()
                .into_owned(),
        ))
    } else {
        Ok(())
    }
}

unsafe fn create_static_variables() -> Result<(), CompilerError> {
    Ok(())
}

unsafe fn create_globals() -> Result<(), CompilerError> {
    Ok(())
}

unsafe fn create_function_heads(ctx: &mut BackendContext) -> Result<(), CompilerError> {
    let nounwind = {
        let name: [i8; 8] = std::mem::transmute(b"nounwind");

        LLVMCreateEnumAttribute(
            LLVMGetGlobalContext(),
            LLVMGetEnumAttributeKindForName(name.as_ptr(), name.len()),
            0,
        )
    };

    for function in ctx.ir_module.functions.values() {
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

        if function.is_foreign {
            LLVMAddAttributeAtIndex(skeleton, LLVMAttributeFunctionIndex, nounwind);
        }

        if !function.is_foreign && !function.is_exposed {
            LLVMSetLinkage(skeleton, LLVMLinkage::LLVMPrivateLinkage);
        }
    }

    Ok(())
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

unsafe fn create_function_bodies(ctx: &mut BackendContext) -> Result<(), CompilerError> {
    Ok(())
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
