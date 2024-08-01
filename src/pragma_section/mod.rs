mod parse;
mod run;

use crate::{
    ast::{AstWorkspace, Source},
    inflow::Inflow,
    parser::Input,
    token::Token,
};

pub struct PragmaSection<'a, I: Inflow<Token>> {
    pub ast: AstWorkspace<'a>,
    pub rest_input: Option<Input<'a, I>>,
    pub pragma_source: Source,
}
