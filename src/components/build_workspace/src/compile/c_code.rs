use ast::AstFile;
use attributes::Privacy;
use build_c_ast::{
    CFileType,
    parse::{Input, Parser},
    translate::TranslateCtx,
    translate_expr,
};
use build_c_token::lex_c_code;
use build_pp_ast::{Preprocessed, preprocess};
use compiler::Compiler;
use diagnostics::{Show, into_show};
use pp_ast::{DefineKind, ObjMacro};
use source_files::{Source, SourceFileKey};
use text::Text;

pub fn c_code(
    compiler: &Compiler,
    text: impl Text,
    key: SourceFileKey,
    c_file_type: CFileType,
) -> Result<AstFile, Box<(dyn Show + 'static)>> {
    let Preprocessed {
        document,
        defines,
        end_of_file,
    } = preprocess(text, compiler.diagnostics).map_err(into_show)?;

    let lexed = lex_c_code(document, end_of_file);

    let mut parser = Parser::new(
        Input::new(lexed, compiler.source_files, key),
        compiler.diagnostics,
        c_file_type,
    );

    parser.parse().map_err(into_show)?;

    // Translate preprocessor #define object macros
    for (define_name, define) in &defines {
        match &define.kind {
            DefineKind::ObjMacro(ObjMacro { replacement, .. }) => {
                parser.switch_input(lex_c_code(replacement.clone(), Source::internal()));

                if let Ok(value) = parser.parse_expr_singular().and_then(|expr| {
                    translate_expr(
                        &mut TranslateCtx {
                            ast_file: &mut parser.ast_file,
                            typedefs: &mut parser.typedefs,
                            diagnostics: compiler.diagnostics,
                        },
                        &expr,
                    )
                }) {
                    parser.ast_file.helper_exprs.push(ast::HelperExpr {
                        name: define_name.clone(),
                        value,
                        source: define.source,
                        is_file_local_only: define.is_file_local_only,
                        privacy: Privacy::Public,
                    });
                }
            }
            DefineKind::FuncMacro(_) => (),
        }
    }

    Ok(parser.ast_file)
}
