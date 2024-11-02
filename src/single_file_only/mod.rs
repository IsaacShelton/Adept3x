/*
    ========================  single_file_only/mod.rs  ========================
    Module for compiling projects that are only a single file
    ---------------------------------------------------------------------------
*/

use crate::{compiler::Compiler, workspace::compile_workspace};
use std::path::Path;

pub fn compile_single_file_only(compiler: &mut Compiler, project_folder: &Path, filepath: &Path) {
    compile_workspace(compiler, project_folder, Some(filepath.to_path_buf()))
}
