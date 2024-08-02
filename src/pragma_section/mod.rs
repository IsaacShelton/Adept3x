mod parse;
mod run;

use crate::{ast::AstWorkspace, inflow::Inflow, parser::Input, source_files::Source, token::Token};

pub struct PragmaSection<'a, I: Inflow<Token>> {
    pub ast: AstWorkspace<'a>,
    pub rest_input: Option<Input<'a, I>>,
    pub pragma_source: Source,
}
