mod parse;
mod run;

use crate::{ast::AstFile, source_files::Source};

pub struct PragmaSection {
    pub ast_file: AstFile,
    pub pragma_source: Source,
}
