use build_llvm_ir::llvm_backend;
use compiler::Compiler;
use diagnostics::unerror;
use std::{
    ffi::OsString,
    fs::create_dir_all,
    path::{Path, PathBuf},
    time::Duration,
};

#[derive(Clone, Debug)]
pub struct ExportDetails {
    pub _linking_duration: Duration,
    pub executable_filepath: PathBuf,
}

pub fn export_and_link(
    compiler: &mut Compiler,
    project_folder: &Path,
    ir_module: &ir::Module,
) -> Result<ExportDetails, ()> {
    let target = &compiler.options.target;
    let project_name = project_name(project_folder);

    let binary_artifacts_folder = project_folder.join("bin");
    let object_files_folder = project_folder.join("obj");
    create_dir_all(&binary_artifacts_folder).expect("failed to create bin folder");
    create_dir_all(&object_files_folder).expect("failed to create obj folder");

    let object_file_filepath =
        object_files_folder.join(target.default_object_file_name(&project_name));

    let executable_filepath =
        binary_artifacts_folder.join(target.default_executable_name(&project_name));

    let linking_duration = unerror(
        unsafe {
            llvm_backend(
                compiler,
                &ir_module,
                &object_file_filepath,
                &executable_filepath,
                &compiler.diagnostics,
            )
        },
        compiler.source_files,
    )?;

    Ok(ExportDetails {
        _linking_duration: linking_duration,
        executable_filepath,
    })
}

fn project_name(project_folder: &Path) -> OsString {
    project_folder
        .file_name()
        .map(OsString::from)
        .or_else(|| {
            std::env::current_dir()
                .ok()
                .and_then(|dir| dir.file_name().map(OsString::from))
        })
        .unwrap_or_else(|| OsString::from("main"))
}
