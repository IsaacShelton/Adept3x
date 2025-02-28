use crate::{
    ast::{self, AstFile, Privacy},
    c::{
        self,
        lexer::lex_c_code,
        preprocessor::{DefineKind, ObjMacro, Preprocessed},
        translate_expr,
    },
    compiler::Compiler,
    show::{into_show, Show},
    source_files::{Source, SourceFileKey},
    text::Text,
};

#[derive(Copy, Clone, Debug)]
pub enum CFileType {
    Header,
    Source,
}

impl CFileType {
    pub fn privacy(&self) -> Privacy {
        match self {
            CFileType::Header => Privacy::Protected,
            CFileType::Source => Privacy::Private,
        }
    }
}

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
    } = c::preprocessor::preprocess(text, compiler.diagnostics).map_err(into_show)?;

    let lexed = lex_c_code(document, end_of_file);

    let mut parser = c::parser::Parser::new(
        c::parser::Input::new(lexed, compiler.source_files, key),
        compiler.diagnostics,
        c_file_type,
    );

    let mut ast_file = parser.parse().map_err(into_show)?;

    // Translate preprocessor #define object macros
    for (define_name, define) in &defines {
        match &define.kind {
            DefineKind::ObjMacro(ObjMacro { replacement, .. }) => {
                parser.switch_input(lex_c_code(replacement.clone(), Source::internal()));

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
            DefineKind::FuncMacro(_) => (),
        }
    }

    Ok(ast_file)
}
