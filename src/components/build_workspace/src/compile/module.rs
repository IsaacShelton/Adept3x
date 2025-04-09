use crate::{module_file::ModuleFile, pragma_section::PragmaSection};
use append_only_vec::AppendOnlyVec;
use ast::AstFile;
use ast_workspace_settings::Settings;
use build_ast::{Input, Parser};
use build_token::Lexer;
use compiler::Compiler;
use data_units::ByteUnits;
use diagnostics::{ErrorDiagnostic, Show, into_show};
use fs_tree::FsNodeId;
use infinite_iterator::{InfiniteIteratorPeeker, InfinitePeekable};
use line_column::Location;
use source_files::{Source, SourceFileKey};
use std::path::Path;
use text::{TextPeeker, TextStreamer};
use token::{Token, TokenKind};

pub struct CompiledModule<'a, I: InfinitePeekable<Token> + 'a> {
    pub total_file_size: ByteUnits,
    pub remaining_input: Input<'a, I>,
    pub settings: Settings,
    pub source_file: SourceFileKey,
}

pub fn compile_module_file<'a>(
    compiler: &Compiler<'a>,
    path: &Path,
) -> Result<CompiledModule<'a, impl InfinitePeekable<Token> + use<'a>>, Box<dyn Show>> {
    let content = std::fs::read_to_string(path)
        .map_err(ErrorDiagnostic::plain)
        .map_err(into_show)?;

    let source_files = &compiler.source_files;
    let key = source_files.add(path.to_path_buf(), content);
    let content = source_files.get(key).content();

    let text = TextPeeker::new(TextStreamer::new(content.chars(), key));
    let lexer = InfiniteIteratorPeeker::new(Lexer::new(text));
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
            "Module file is missing pragma section, consider adding `pragma => adept(\"3.0\")` at the top of your file",
            Source {
                key,
                location: Location { line: 1, column: 1 },
            },
        )));
    };

    Ok(CompiledModule {
        total_file_size: ByteUnits::of(content.len().try_into().unwrap()),
        remaining_input: input,
        settings,
        source_file: key,
    })
}

pub fn compile_rest_module_file<'a, I: InfinitePeekable<Token>>(
    module_file: &ModuleFile,
    rest_input: Input<'a, I>,
    out_ast_files: &AppendOnlyVec<(FsNodeId, AstFile)>,
) -> Result<ByteUnits, Box<(dyn Show + 'static)>> {
    let mut parser = Parser::new(rest_input);
    out_ast_files.push((module_file.fs_node_id, parser.parse().map_err(into_show)?));
    Ok(ByteUnits::ZERO) // No new bytes processed
}
