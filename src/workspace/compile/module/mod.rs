use crate::{
    compiler::Compiler,
    diagnostics::ErrorDiagnostic,
    inflow::{Inflow, IntoInflow},
    lexer::Lexer,
    parser::Input,
    pragma_section::PragmaSection,
    show::{into_show, Show},
    text::{IntoText, IntoTextStream},
    token::{Token, TokenKind},
    workspace::fs::Fs,
};
use std::path::Path;

pub fn compile_module_file<'a>(
    compiler: &Compiler<'a>,
    _fs: &Fs,
    path: &Path,
) -> Result<(usize, Input<'a, impl Inflow<Token> + 'a>), Box<dyn Show + 'a>> {
    let content = std::fs::read_to_string(path)
        .map_err(ErrorDiagnostic::plain)
        .map_err(into_show)?;

    let source_files = &compiler.source_files;
    let key = source_files.add(path.to_path_buf(), content);
    let content = source_files.get(key).content();

    let text = content.chars().into_text_stream(key).into_text();
    let lexer = Lexer::new(text).into_inflow();
    let mut input = Input::new(lexer, compiler.source_files, key);
    input.ignore_newlines();

    while input.peek_is(TokenKind::PragmaKeyword) {
        let (section, rest_input) = PragmaSection::parse(input)?;
        input = rest_input;
        section.run(compiler, path)?;
        input.ignore_newlines();
    }

    Ok((content.len(), input))
}
