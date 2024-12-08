use super::{file::CodeFile, fs::FsNodeId, module_file::ModuleFile, normal_file::NormalFile};
use crate::{
    ast::{AstFile, Settings},
    inflow::Inflow,
    token::Token,
};
use append_only_vec::AppendOnlyVec;
use itertools::Itertools;
use std::sync::Mutex;

pub struct WorkspaceQueue<'a, I: Inflow<Token>> {
    pub ast_files: AppendOnlyVec<(FsNodeId, AstFile)>,
    pub module_folders: AppendOnlyVec<(FsNodeId, Settings)>,
    code_files: Mutex<Vec<CodeFile<'a, I>>>,
    module_files: Mutex<Vec<ModuleFile>>,
}

impl<'a, I: Inflow<Token>> WorkspaceQueue<'a, I> {
    pub fn new(normal_files: Vec<NormalFile>, module_files: Vec<ModuleFile>) -> Self {
        Self {
            ast_files: AppendOnlyVec::new(),
            module_folders: AppendOnlyVec::new(),
            code_files: Mutex::new(normal_files.into_iter().map(CodeFile::Normal).collect_vec()),
            module_files: Mutex::new(module_files),
        }
    }

    pub fn push_module_folder(&self, folder_fs_node_id: FsNodeId, settings: Settings) {
        self.module_folders.push((folder_fs_node_id, settings));
    }

    pub fn push_code_file(&self, code_file: CodeFile<'a, I>) {
        self.code_files.lock().unwrap().push(code_file);
    }

    pub fn push_code_files(&self, code_files: impl Iterator<Item = CodeFile<'a, I>>) {
        self.code_files.lock().unwrap().extend(code_files);
    }

    pub fn push_module_files(&self, module_files: impl Iterator<Item = ModuleFile>) {
        self.module_files.lock().unwrap().extend(module_files);
    }

    pub fn for_module_files(&self, f: impl Fn(ModuleFile)) {
        loop {
            // CAREFUL: Lock doesn't immediately drop unless we do it this way (while loop is not equivalent)
            let Some(module_file) = self.module_files.lock().unwrap().pop() else {
                break;
            };
            f(module_file);
        }
    }

    pub fn for_code_files(&self, f: impl Fn(CodeFile<'a, I>)) {
        loop {
            // CAREFUL: Lock doesn't immediately drop unless we do it this way (while loop is not equivalent)
            let Some(code_file) = self.code_files.lock().unwrap().pop() else {
                break;
            };
            f(code_file);
        }
    }
}
