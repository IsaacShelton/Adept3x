use super::{
    annotation::Annotation,
    error::{ParseError, ParseErrorKind},
    Parser,
};
use crate::{
    ast::{AstFile, Enum, HelperExpr, Named, TypeAlias},
    index_map_ext::IndexMapExt,
    inflow::Inflow,
    token::{Token, TokenKind},
};

impl<'a, I: Inflow<Token>> Parser<'a, I> {
    pub fn parse_top_level(
        &mut self,
        ast_file: &mut AstFile,
        parent_annotations: Vec<Annotation>,
    ) -> Result<(), ParseError> {
        let mut annotations = parent_annotations;

        // Ignore preceeding newlines
        self.ignore_newlines();

        // Parse annotations
        while self.input.peek().is_hash() {
            annotations.push(self.parse_annotation()?);
            self.ignore_newlines();
        }

        // Ignore newlines after annotations
        self.ignore_newlines();

        // Parse top-level construct
        match self.input.peek().kind {
            TokenKind::OpenCurly => {
                self.input.advance().kind.unwrap_open_curly();
                self.ignore_newlines();

                while !self.input.peek_is_or_eof(TokenKind::CloseCurly) {
                    self.parse_top_level(ast_file, annotations.clone())?;
                    self.ignore_newlines();
                }

                self.parse_token(TokenKind::CloseCurly, Some("to close annotation group"))?;
            }
            TokenKind::FuncKeyword => {
                ast_file.functions.push(self.parse_function(annotations)?);
            }
            TokenKind::Identifier(_) => {
                ast_file
                    .global_variables
                    .push(self.parse_global_variable(annotations)?);
            }
            TokenKind::StructKeyword => {
                ast_file.structures.push(self.parse_structure(annotations)?)
            }
            TokenKind::TypeAliasKeyword => {
                let Named::<TypeAlias> { name, value: alias } =
                    self.parse_type_alias(annotations)?;
                let source = alias.source;

                ast_file.type_aliases.try_insert(name, alias, |name| {
                    ParseErrorKind::TypeAliasHasMultipleDefinitions { name }.at(source)
                })?;
            }
            TokenKind::EnumKeyword => {
                let Named::<Enum> {
                    name,
                    value: enum_definition,
                } = self.parse_enum(annotations)?;

                let source = enum_definition.source;

                ast_file.enums.try_insert(name, enum_definition, |name| {
                    ParseErrorKind::EnumHasMultipleDefinitions { name }.at(source)
                })?;
            }
            TokenKind::DefineKeyword => {
                let Named::<HelperExpr> {
                    name,
                    value: named_expr,
                } = self.parse_helper_expr(annotations)?;
                let source = named_expr.source;

                ast_file.helper_exprs.try_insert(name, named_expr, |name| {
                    ParseErrorKind::DefineHasMultipleDefinitions { name }.at(source)
                })?;
            }
            TokenKind::EndOfFile => {
                // End-of-file is only okay if no preceeding annotations
                if !annotations.is_empty() {
                    let token = self.input.advance();
                    return Err(self.expected_top_level_construct(&token));
                }
            }
            _ => {
                return Err(self.unexpected_token_is_next());
            }
        }

        Ok(())
    }
}