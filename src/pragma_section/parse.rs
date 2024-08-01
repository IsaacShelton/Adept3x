use super::PragmaSection;
use crate::{
    ast::{AstWorkspace, Function, Parameters, StmtKind, TypeKind},
    inflow::Inflow,
    parser::{self, error::ParseError, Input},
    show::{into_show, Show},
    token::{Token, TokenKind},
};

impl<'a, I: Inflow<Token> + 'a> PragmaSection<'a, I> {
    pub fn parse(mut input: Input<'a, I>) -> Result<PragmaSection<'a, I>, Box<dyn Show>> {
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

        let mut ast = AstWorkspace::new(input.source_file_cache());
        let mut ast_file = ast.new_file();
        let mut parser = parser::Parser::new(input);

        if parser.input.eat(TokenKind::OpenCurly) {
            // "Whole-file" mode

            // Parse top-level contructs until we hit a '}'
            while !parser.input.peek_is(TokenKind::CloseCurly) {
                parser
                    .parse_top_level(&mut ast_file, vec![])
                    .map_err(into_show)?;
                parser.input.ignore_newlines();
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

            ast_file.functions.push(Function {
                name: "main".into(),
                parameters: Parameters {
                    required: vec![],
                    is_cstyle_vararg: false,
                },
                return_type: TypeKind::Void.at(source),
                stmts,
                is_foreign: false,
                source,
                abide_abi: false,
                tag: None,
            });
        } else {
            return Err(Box::new(ParseError::expected(
                "'=>' or '{' after 'pragma' keyword",
                None::<&str>,
                parser.input.peek(),
            )));
        }

        // Leave input unfinished
        input = parser.input;

        Ok(PragmaSection {
            ast,
            rest_input: Some(input),
            pragma_source,
        })
    }
}
