use super::PragmaSection;
use crate::{
    ast::{
        AstFile, Expr, ExprKind, Func, FuncHead, Params, Privacy, Stmt, StmtKind, TypeKind,
        TypeParams,
    },
    diagnostics::ErrorDiagnostic,
    inflow::Inflow,
    parser::{self, error::ParseError, Input},
    show::{into_show, Show},
    source_files::Source,
    token::{Token, TokenKind},
};

impl PragmaSection {
    pub fn parse<'a, I: Inflow<Token>>(
        allow_experimental_pragma_features: bool,
        mut input: Input<'a, I>,
        require_adept_version_first: bool,
    ) -> Result<(PragmaSection, Input<'a, I>), Box<dyn Show>> {
        // pragma ...
        //   ^

        input.ignore_newlines();

        let Some(pragma_source) = input.eat_remember(TokenKind::PragmaKeyword) else {
            return Err(Box::new(ParseError::expected(
                "Expected 'pragma' at beginning of module file",
                None::<&str>,
                input.peek(),
            )));
        };

        input.ignore_newlines();

        let mut parser = parser::Parser::new_for_pragma(input);
        let mut ast_file = AstFile::new();

        if parser.input.eat(TokenKind::OpenCurly) {
            // "Whole-file" mode

            // Parse top-level contructs until we hit a '}'
            if allow_experimental_pragma_features {
                while !parser.input.peek_is_or_eof(TokenKind::CloseCurly) {
                    parser
                        .parse_top_level(&mut ast_file, vec![])
                        .map_err(into_show)?;
                    parser.input.ignore_newlines();
                }
            } else {
                return Err(Box::new(ErrorDiagnostic::new(
                    "Whole-file pragma directives are an experimental feature and may be removed",
                    pragma_source,
                )));
            }

            // Eat the final '}'
            if !parser.input.eat(TokenKind::CloseCurly) {
                return Err(Box::new(ParseError::expected(
                    "'}'",
                    Some("to close pragma section"),
                    parser.input.peek(),
                )));
            }
        } else if let Some(source) = parser.input.eat_remember(TokenKind::FatArrow) {
            // Determine if should parse block or just single expression
            let is_block = parser.input.peek_is(TokenKind::OpenCurly);

            let stmts = if is_block {
                // "Inside-main-only" mode
                parser.parse_block("pragma").map_err(into_show)?
            } else {
                // "Single-expression" mode
                let expr = parser.parse_expr().map_err(into_show)?;
                let expr_source = expr.source;
                vec![StmtKind::Expr(expr).at(expr_source)]
            };

            if require_adept_version_first {
                let has_adept_first = stmts.first().map_or(false, |stmt| {
                    if let StmtKind::Expr(e) = &stmt.kind {
                        if let ExprKind::Call(c) = &e.kind {
                            c.name.as_plain_str() == Some("adept")
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                });

                if !has_adept_first {
                    return Err(Box::new(ErrorDiagnostic::new(
                        "First 'pragma' section must start with adept(\"3.0\") statement, i.e. pragma => adept(\"3.0\")",
                        pragma_source,
                    )));
                }
            }

            if !allow_experimental_pragma_features {
                for stmt in stmts.iter() {
                    restrict_allowed_stmt(stmt)?;
                }
            }

            ast_file.funcs.push(Func {
                head: FuncHead {
                    name: "main".into(),
                    type_params: TypeParams::default(),
                    givens: vec![],
                    params: Params::default(),
                    return_type: TypeKind::Void.at(source),
                    is_foreign: false,
                    source,
                    abide_abi: false,
                    tag: None,
                    privacy: Privacy::Public,
                },
                stmts,
            });
        } else {
            return Err(Box::new(ParseError::expected(
                "'=>' or '{' after 'pragma' keyword",
                None::<&str>,
                parser.input.peek(),
            )));
        }

        Ok((
            PragmaSection {
                ast_file,
                pragma_source,
            },
            parser.input,
        ))
    }
}

fn restrict_allowed_stmt(stmt: &Stmt) -> Result<(), Box<dyn Show>> {
    match &stmt.kind {
        StmtKind::Expr(inner) => restrict_allowed_expr(inner)?,
        _ => return Err(found_forbidden_stmt(stmt.source)),
    }

    Ok(())
}

fn restrict_allowed_expr(expr: &Expr) -> Result<(), Box<dyn Show>> {
    match &expr.kind {
        ExprKind::NullTerminatedString(_) => (),
        ExprKind::Call(call) => {
            if call.expected_to_return.is_some() {
                return Err(found_forbidden_expr(expr.source));
            }

            for argument in call.args.iter() {
                restrict_allowed_expr(argument)?;
            }
        }
        _ => return Err(found_forbidden_expr(expr.source)),
    }

    Ok(())
}

fn found_forbidden_stmt(source: Source) -> Box<dyn Show> {
    Box::new(ErrorDiagnostic::new(
        "Support for this statement inside pragma directives is an experimental feature that may be removed",
        source,
    ))
}

fn found_forbidden_expr(source: Source) -> Box<dyn Show> {
    Box::new(ErrorDiagnostic::new(
        "Support for this expression inside pragma directives is an experimental feature that may be removed",
        source,
    ))
}
