use crate::{
    ast::Settings,
    compiler::Compiler,
    diagnostics::ErrorDiagnostic,
    inflow::{Inflow, IntoInflow},
    lexer::Lexer,
    line_column::Location,
    parser::Input,
    pragma_section::PragmaSection,
    show::{into_show, Show},
    source_files::Source,
    text::{IntoText, IntoTextStream},
    token::{Token, TokenKind},
    workspace::fs::Fs,
};
use std::path::Path;

pub fn compile_module_file<'a>(
    compiler: &Compiler<'a>,
    _fs: &Fs,
    path: &Path,
) -> Result<(usize, Input<'a, impl Inflow<Token> + 'a>, Settings), Box<dyn Show + 'a>> {
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

    let mut settings = None;

    while input.peek_is(TokenKind::PragmaKeyword) {
        let (section, rest_input) = PragmaSection::parse(
            compiler.options.allow_experimental_pragma_features,
            input,
            settings.is_none(),
        )?;
        input = rest_input;
        settings = Some(section.run(compiler, path, settings)?);
        input.ignore_newlines();
    }

    let Some(settings) = settings else {
        return Err(Box::new(ErrorDiagnostic::new(
            "Module file is missing pragma section",
            Source {
                key,
                location: Location { line: 1, column: 1 },
            },
        )));
    };

    Ok((content.len(), input, settings))
}
