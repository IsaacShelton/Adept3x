use crate::{file::CodeFile, module_file::ModuleFile, normal_file::NormalFile};
use append_only_vec::AppendOnlyVec;
use ast::RawAstFile;
use ast_workspace_settings::Settings;
use fs_tree::FsNodeId;
use infinite_iterator::InfinitePeekable;
use std::{
    collections::{HashMap, VecDeque},
    sync::Mutex,
};
use token::Token;

pub struct LexParseQueue<'a, I: InfinitePeekable<Token>> {
    pub ast_files: AppendOnlyVec<(FsNodeId, RawAstFile)>,
    module_folders: AppendOnlyVec<(FsNodeId, Settings)>,
    code_files: Mutex<VecDeque<CodeFile<'a, I>>>,
    module_files: Mutex<VecDeque<ModuleFile>>,
}

pub struct LexParseInfo {
    pub module_folders: HashMap<FsNodeId, Settings>,
    pub files: HashMap<FsNodeId, RawAstFile>,
}

impl<'a, I: InfinitePeekable<Token>> LexParseQueue<'a, I> {
    pub fn new(normal_files: Vec<NormalFile>, module_files: Vec<ModuleFile>) -> Self {
        Self {
            ast_files: AppendOnlyVec::new(),
            module_folders: AppendOnlyVec::new(),
            code_files: Mutex::new(normal_files.into_iter().map(CodeFile::Normal).collect()),
            module_files: Mutex::new(module_files.into()),
        }
    }

    pub fn push_module_folder(&self, folder_fs_node_id: FsNodeId, settings: Settings) {
        self.module_folders.push((folder_fs_node_id, settings));
    }

    pub fn enqueue_code_file(&self, code_file: CodeFile<'a, I>) {
        self.code_files.lock().unwrap().push_back(code_file);
    }

    pub fn enqueue_code_files(&self, code_files: impl Iterator<Item = CodeFile<'a, I>>) {
        self.code_files.lock().unwrap().extend(code_files);
    }

    pub fn enqueue_module_files(&self, module_files: impl Iterator<Item = ModuleFile>) {
        self.module_files.lock().unwrap().extend(module_files);
    }

    pub fn module_folders_so_far(&self) -> HashMap<FsNodeId, &Settings> {
        HashMap::from_iter(
            self.module_folders
                .iter()
                .map(|(fs_node_id, settings)| (*fs_node_id, settings)),
        )
    }

    pub fn destructure(self) -> LexParseInfo {
        let module_folders = HashMap::from_iter(self.module_folders.into_iter());
        let files = HashMap::from_iter(self.ast_files.into_iter());

        LexParseInfo {
            module_folders,
            files,
        }
    }

    pub fn for_module_files(&self, f: impl Fn(ModuleFile)) {
        loop {
            // CAREFUL: Lock doesn't immediately drop unless we do it this way (while loop is not equivalent)
            let Some(module_file) = self.module_files.lock().unwrap().pop_front() else {
                break;
            };
            f(module_file);
        }
    }

    pub fn for_code_files(&self, f: impl Fn(CodeFile<'a, I>)) {
        loop {
            // CAREFUL: Lock doesn't immediately drop unless we do it this way (while loop is not equivalent)
            let Some(code_file) = self.code_files.lock().unwrap().pop_front() else {
                break;
            };
            f(code_file);
        }
    }
}
