use super::{module_file::ModuleFile, normal_file::NormalFile};
use build_ast::Input;
use derive_more::IsVariant;
use inflow::Inflow;
use std::path::Path;
use token::Token;

#[derive(IsVariant)]
pub enum CodeFile<'a, I: Inflow<Token>> {
    Normal(NormalFile),
    Module(ModuleFile, Input<'a, I>),
}

impl<'a, I: Inflow<Token>> CodeFile<'a, I> {
    pub fn path(&self) -> &Path {
        match self {
            CodeFile::Normal(normal_file) => &normal_file.path,
            CodeFile::Module(module_file, _) => &module_file.path,
        }
    }
}
