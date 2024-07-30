mod ast;
mod error;
mod expand;
mod lexer;
mod line_splice;
mod parser;
mod pre_token;
mod stdc;

use self::error::PreprocessorError;
use self::expand::expand_ast;
use self::lexer::Lexer;
use self::parser::{parse, ParseErrorKind};
use self::stdc::stdc;
use crate::ast::Source;
use crate::diagnostics::Diagnostics;
use crate::inflow::IntoInflow;
use crate::text::Text;
use std::collections::HashMap;

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

pub use self::ast::{Define, DefineKind};
pub use self::pre_token::{PreToken, PreTokenKind};

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
    let ast = parse(lexer.into_inflow(), diagnostics)?;
    let (document, defines) = expand_ast(&ast, stdc())?;

    Ok(Preprocessed {
        document,
        defines,
        end_of_file: ast.eof,
    })
}
