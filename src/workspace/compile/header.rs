use crate::{
    ast::{self, AstFile, Privacy},
    c::{
        self,
        lexer::lex_c_code,
        preprocessor::{DefineKind, Preprocessed},
        translate_expr,
    },
    compiler::Compiler,
    show::{into_show, Show},
    source_files::{Source, SourceFileKey},
    text::Text,
};

pub fn header(
    compiler: &Compiler,
    text: impl Text,
    key: SourceFileKey,
) -> Result<AstFile, Box<(dyn Show + 'static)>> {
    let Preprocessed {
        document,
        defines,
        end_of_file,
    } = c::preprocessor::preprocess(text, compiler.diagnostics).map_err(into_show)?;

    let lexed = lex_c_code(document, end_of_file);

    let mut parser = c::parser::Parser::new(
        c::parser::Input::new(lexed, compiler.source_files, key),
        compiler.diagnostics,
    );

    let mut ast_file = parser.parse().map_err(into_show)?;

    // Translate preprocessor #define object macros
    for (define_name, define) in &defines {
        match &define.kind {
            DefineKind::ObjectMacro(expanded_replacement, _placeholder_affinity) => {
                let lexed_replacement =
                    lex_c_code(expanded_replacement.clone(), Source::internal());
                parser.switch_input(lexed_replacement);

                if let Ok(value) = parser.parse_expr_singular().and_then(|expr| {
                    translate_expr(
                        &mut ast_file,
                        parser.typedefs(),
                        &expr,
                        compiler.diagnostics,
                    )
                }) {
                    ast_file.helper_exprs.push(ast::HelperExpr {
                        name: define_name.clone(),
                        value,
                        source: define.source,
                        is_file_local_only: define.is_file_local_only,
                        privacy: Privacy::Public,
                    });
                }
            }
            DefineKind::FunctionMacro(_) => (),
        }
    }

    Ok(ast_file)
}
