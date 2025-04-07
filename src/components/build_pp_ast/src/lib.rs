mod error;
mod expand;
mod lexer;
mod line_splice;
mod parser;
mod stdc;

/*
   Missing features:
   - __has_include
   - __has_embed
   - __has_c_attribute
   - #embed (and its options)
   - #pragma STDC (all of its options)
   - __FILE__
   - __LINE__
   - __DATE__
   - etc.
*/
use self::{error::PreprocessorError, expand::expand_ast, lexer::Lexer, parser::parse, stdc::stdc};
use diagnostics::Diagnostics;
use infinite_iterator::InfiniteIteratorPeeker;
use pp_ast::Define;
use pp_token::PreToken;
use source_files::Source;
use std::collections::HashMap;
use text::Text;

#[derive(Clone, Debug)]
pub struct Preprocessed {
    pub document: Vec<PreToken>,
    pub defines: HashMap<String, Define>,
    pub end_of_file: Source,
}

pub fn preprocess(
    text: impl Text,
    diagnostics: &Diagnostics,
) -> Result<Preprocessed, PreprocessorError> {
    let lexer = Lexer::new(text);
    let ast = parse(InfiniteIteratorPeeker::new(lexer), diagnostics)?;
    let (document, defines) = expand_ast(&ast, stdc())?;

    Ok(Preprocessed {
        document,
        defines,
        end_of_file: ast.eof,
    })
}
