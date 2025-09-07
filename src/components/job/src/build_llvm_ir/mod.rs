#![allow(unsafe_op_in_unsafe_fn)]

mod abi;
mod address;
mod backend_type;
mod builder;
mod ctx;
mod functions;
mod globals;
mod intrinsics;
mod llvm_type_ref_ext;
mod llvm_value_ref_ext;
mod module;
mod null_terminated_string;
mod raw_address;
mod structure;
mod target_data;
mod target_machine;
mod target_triple;
mod value_catalog;
mod values;

use self::{
    ctx::BackendCtx,
    functions::{body::create_function_bodies, head::create_func_heads},
    globals::{create_globals, create_static_variables},
    module::BackendModule,
    target_data::TargetData,
    target_machine::TargetMachine,
    target_triple::{get_triple, make_llvm_target},
};
use crate::{
    ExecutionCtx, build_executable::link_result, ir, module_graph::ModuleGraphMeta, repr::Compiler,
};
use colored::Colorize;
use compiler::BuildOptions;
use diagnostics::{Diagnostics, ErrorDiagnostic};
use llvm_sys::{
    analysis::{LLVMVerifierFailureAction::LLVMPrintMessageAction, LLVMVerifyModule},
    core::LLVMPrintModuleToString,
    target::{
        LLVM_InitializeAllAsmParsers, LLVM_InitializeAllAsmPrinters, LLVM_InitializeAllTargetInfos,
        LLVM_InitializeAllTargetMCs, LLVM_InitializeAllTargets, LLVMSetModuleDataLayout,
    },
    target_machine::{
        LLVMCodeGenFileType, LLVMCodeGenOptLevel, LLVMCodeModel, LLVMRelocMode,
        LLVMTargetMachineEmitToFile,
    },
};
use std::{
    ffi::{CStr, CString, c_char},
    path::Path,
    ptr::null_mut,
    time::Duration,
};
use target::TargetOs;

pub unsafe fn llvm_backend<'env>(
    ctx: &mut ExecutionCtx<'env>,
    compiler: &mut Compiler<'env>,
    options: &BuildOptions,
    ir_module: &'env ir::Ir<'env>,
    meta: &ModuleGraphMeta,
    output_object_filepath: &Path,
    output_binary_filepath: &Path,
    diagnostics: &'env Diagnostics,
) -> Result<Duration, ErrorDiagnostic> {
    LLVM_InitializeAllTargetInfos();
    LLVM_InitializeAllTargets();
    LLVM_InitializeAllTargetMCs();
    LLVM_InitializeAllAsmParsers();
    LLVM_InitializeAllAsmPrinters();

    let module_name = CString::new(output_object_filepath.to_str().expect("valid utf8")).unwrap();
    let triple = get_triple(&options.target)?;
    let target = make_llvm_target(&triple)?;
    let cpu = CString::new("generic").unwrap();
    let features = CString::new("").unwrap();
    let level = LLVMCodeGenOptLevel::LLVMCodeGenLevelDefault;
    let code_model = LLVMCodeModel::LLVMCodeModelDefault;

    let reloc = if options
        .use_pic
        .unwrap_or_else(|| matches!(meta.target.os(), Some(TargetOs::Linux | TargetOs::Mac)))
    {
        LLVMRelocMode::LLVMRelocPIC
    } else {
        LLVMRelocMode::LLVMRelocDefault
    };

    let backend_module = BackendModule::new(&module_name);
    let target_machine =
        TargetMachine::new(target, &triple, &cpu, &features, level, reloc, code_model);
    let target_data = TargetData::new(&target_machine);
    LLVMSetModuleDataLayout(backend_module.get(), target_data.get());

    let mut ctx = BackendCtx::new(
        ctx,
        ir_module,
        meta,
        &backend_module,
        &target_data,
        diagnostics,
    )?;

    create_static_variables()?;
    create_globals(&mut ctx)?;
    create_func_heads(&mut ctx)?;
    create_function_bodies(&mut ctx)?;

    if options.emit_llvm_ir {
        use std::{fs::File, io::Write};

        let mut f = File::create("out.ll").expect("failed to emit llvm ir to file");

        writeln!(
            &mut f,
            "{}",
            CStr::from_ptr(LLVMPrintModuleToString(backend_module.get()))
                .to_str()
                .expect("valid utf-8 llvm ir")
        )
        .expect("failed to write llvm ir to file");
    }

    if LLVMVerifyModule(backend_module.get(), LLVMPrintMessageAction, null_mut()) == 1 {
        println!(
            "{}",
            "\n---- WARNING: llvm module verification failed! ----".yellow()
        );
    }
    let output_object_filename =
        CString::new(output_object_filepath.to_str().expect("valid utf8")).unwrap();

    let mut llvm_emit_error_message: *mut c_char = null_mut();
    LLVMTargetMachineEmitToFile(
        target_machine.get(),
        backend_module.get(),
        output_object_filename.into_raw(),
        LLVMCodeGenFileType::LLVMObjectFile,
        &mut llvm_emit_error_message,
    );

    if !llvm_emit_error_message.is_null() {
        return Err(ErrorDiagnostic::plain(
            CString::from_raw(llvm_emit_error_message).to_string_lossy(),
        ));
    }

    link_result(
        compiler,
        options,
        &meta.target,
        diagnostics,
        output_object_filepath,
        output_binary_filepath,
    )
}
