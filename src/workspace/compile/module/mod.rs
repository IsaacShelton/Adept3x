use crate::{
    compiler::Compiler,
    inflow::IntoInflow,
    lexer::Lexer,
    parser::Input,
    pragma_section::PragmaSection,
    text::{IntoTextStream, TextStream},
    token::TokenKind,
    workspace::fs::Fs,
};
use std::path::Path;

pub fn compile_module_file(compiler: &Compiler, _fs: &Fs, path: &Path) -> Result<usize, ()> {
    let content = std::fs::read_to_string(path).map_err(|err| {
        eprintln!("{}", err);
        ()
    })?;

    let source_file_cache = &compiler.source_file_cache;
    let key = source_file_cache.add(path.to_path_buf(), content);
    let content = source_file_cache.get(key).content();

    let text = content.chars().into_text_stream(key).into_text();
    let lexer = Lexer::new(text).into_inflow();
    let mut input = Input::new(lexer, compiler.source_file_cache, key);
    input.ignore_newlines();

    while input.peek_is(TokenKind::PragmaKeyword) {
        input = match PragmaSection::parse(input).and_then(|section| section.run(compiler)) {
            Ok(Some(rest)) => rest,
            Ok(None) => break,
            Err(err) => {
                let mut s = String::new();
                err.show(&mut s, source_file_cache).unwrap();
                eprintln!("{}", s);
                return Err(());
            }
        };

        input.ignore_newlines();
    }

    Ok(content.len())
}
