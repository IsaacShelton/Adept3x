use super::{module_file::ModuleFile, normal_file::NormalFile};
use build_ast::Input;
use derive_more::IsVariant;
use infinite_iterator::InfinitePeekable;
use std::path::Path;
use token::Token;

#[derive(IsVariant)]
pub enum CodeFile<'a, I: InfinitePeekable<Token>> {
    Normal(NormalFile),
    Module(ModuleFile, Input<'a, I>),
}

impl<'a, I: InfinitePeekable<Token>> CodeFile<'a, I> {
    pub fn path(&self) -> &Path {
        match self {
            CodeFile::Normal(normal_file) => &normal_file.path,
            CodeFile::Module(module_file, _) => &module_file.path,
        }
    }
}
