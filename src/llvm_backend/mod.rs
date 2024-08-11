mod abi;
mod address;
mod backend_type;
mod builder;
mod ctx;
mod error;
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
mod variable_stack;

use self::{
    ctx::BackendCtx,
    error::BackendError,
    functions::{body::create_function_bodies, head::create_function_heads},
    globals::{create_globals, create_static_variables},
    module::BackendModule,
    target_data::TargetData,
    target_machine::TargetMachine,
    target_triple::{get_target_from_triple, get_triple},
};
use crate::{
    compiler::Compiler,
    diagnostics::{Diagnostics, WarningDiagnostic},
    ir, resolved,
};
use colored::Colorize;
use llvm_sys::{
    analysis::{LLVMVerifierFailureAction::LLVMPrintMessageAction, LLVMVerifyModule},
    core::LLVMPrintModuleToString,
    target::{
        LLVMSetModuleDataLayout, LLVM_InitializeAllAsmParsers, LLVM_InitializeAllAsmPrinters,
        LLVM_InitializeAllTargetInfos, LLVM_InitializeAllTargetMCs, LLVM_InitializeAllTargets,
    },
    target_machine::{LLVMCodeGenFileType, LLVMCodeGenOptLevel, LLVMCodeModel, LLVMRelocMode},
};
use std::{
    ffi::{c_char, CStr, CString, OsStr, OsString},
    path::Path,
    process::Command,
    ptr::null_mut,
    time::{Duration, Instant},
};

pub unsafe fn llvm_backend(
    compiler: &mut Compiler,
    ir_module: &ir::Module,
    resolved_ast: &resolved::Ast,
    output_object_filepath: &Path,
    output_binary_filepath: &Path,
    diagnostics: &Diagnostics,
) -> Result<Duration, BackendError> {
    LLVM_InitializeAllTargetInfos();
    LLVM_InitializeAllTargets();
    LLVM_InitializeAllTargetMCs();
    LLVM_InitializeAllAsmParsers();
    LLVM_InitializeAllAsmPrinters();

    let options = &compiler.options;
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

    let mut ctx = BackendCtx::new(
        ir_module,
        &backend_module,
        &target_data,
        resolved_ast,
        diagnostics,
    );

    create_static_variables()?;
    create_globals(&mut ctx)?;
    create_function_heads(&mut ctx)?;
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

    let output_object_filename =
        CString::new(output_object_filepath.to_str().expect("valid utf8")).unwrap();

    if LLVMVerifyModule(backend_module.get(), LLVMPrintMessageAction, null_mut()) == 1 {
        println!(
            "{}",
            "\n---- WARNING: llvm module verification failed! ----".yellow()
        );
    }

    let mut llvm_emit_error_message: *mut c_char = null_mut();

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

    let mut args = vec![
        output_object_filepath.as_os_str().into(),
        OsString::from("-o"),
        output_binary_filepath.as_os_str().into(),
    ];

    for (filename, _) in compiler.link_filenames.iter_mut() {
        if is_flag_like(filename) {
            eprintln!("warning: ignoring incorrect link filename '{}'", filename);
        } else {
            args.push(OsString::from(filename));
        }
    }

    for (framework, _) in compiler.link_frameworks.iter_mut() {
        args.push(OsString::from("-framework"));
        args.push(OsString::from(framework));
    }

    if ir_module.target_info.kind.is_arbitrary() {
        let args = args.join(OsStr::new(" "));

        diagnostics.push(WarningDiagnostic::plain(
            format!(
                "Automatic linking is not supported yet on your system, please link manually with something like:\n gcc {}",
                args.to_string_lossy()
            )
        ));

        eprintln!("Success, but requires manual linking, exiting with 1");
        std::process::exit(1);
    } else {
        let start_time = Instant::now();

        // Link resulting object file to create executable
        let mut command = Command::new("gcc")
            .args(args)
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

        Ok(start_time.elapsed())
    }
}

fn is_flag_like(string: &str) -> bool {
    string.chars().skip_while(|c| c.is_whitespace()).next() == Some('-')
}
