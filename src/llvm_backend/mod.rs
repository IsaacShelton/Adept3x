mod abi;
mod backend_type;
mod builder;
mod ctx;
mod error;
mod functions;
mod globals;
mod intrinsics;
mod module;
mod null_terminated_string;
mod structure;
mod target_data;
mod target_machine;
mod target_triple;
mod value_catalog;
mod values;
mod variable_stack;

use self::{
    abi::{
        abi_function::ABIFunction,
        arch::{aarch64, Arch},
    },
    ctx::BackendContext,
    error::BackendError,
    functions::{body::create_function_bodies, head::create_function_heads},
    globals::{create_globals, create_static_variables},
    module::BackendModule,
    target_data::TargetData,
    target_machine::TargetMachine,
    target_triple::{get_target_from_triple, get_triple},
};
use crate::{ir, target_info::type_info::TypeInfoManager};
use colored::Colorize;
use llvm_sys::{
    analysis::{LLVMVerifierFailureAction::LLVMPrintMessageAction, LLVMVerifyModule},
    target::{
        LLVMSetModuleDataLayout, LLVM_InitializeAllAsmParsers, LLVM_InitializeAllAsmPrinters,
        LLVM_InitializeAllTargetInfos, LLVM_InitializeAllTargetMCs, LLVM_InitializeAllTargets,
    },
    target_machine::{LLVMCodeGenFileType, LLVMCodeGenOptLevel, LLVMCodeModel, LLVMRelocMode},
};
use std::{
    ffi::{c_char, CString, OsStr},
    path::Path,
    process::Command,
    ptr::null_mut,
};

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

    // TODO: Use abi translations for declaring/calling functions
    if false {
        let _abi_function = ABIFunction::new(
            Arch::AARCH64(aarch64::AARCH64 {
                variant: aarch64::Variant::DarwinPCS,
                target_info: &ir_module.target_info,
                type_info_manager: &TypeInfoManager::new(),
                ir_module,
            }),
            &vec![],
            &ir::Type::S8,
            false,
        );
    }

    let mut llvm_emit_error_message: *mut c_char = null_mut();

    // Print generated LLVM IR?
    // println!("{}", CStr::from_ptr(LLVMPrintModuleToString(backend_module.get())).to_string_lossy());

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
