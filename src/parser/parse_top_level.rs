use super::{
    annotation::{Annotation, AnnotationKind},
    error::ParseError,
    Parser,
};
use crate::{
    ast::AstFile,
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
            annotations.extend(self.parse_annotation()?);
            self.ignore_newlines();
        }

        // Parse pub keyword
        if self.input.peek().is_pub_keyword() {
            annotations.push(AnnotationKind::Public.at(self.input.advance().source));
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
                let type_alias = self.parse_type_alias(annotations)?;
                ast_file.type_aliases.push(type_alias);
            }
            TokenKind::EnumKeyword => {
                let enum_definition = self.parse_enum(annotations)?;

                ast_file.enums.push(enum_definition);
            }
            TokenKind::DefineKeyword => {
                let helper_expr = self.parse_helper_expr(annotations)?;
                ast_file.helper_exprs.push(helper_expr);
            }
            TokenKind::TraitKeyword => {
                let trait_decl = self.parse_trait(annotations)?;
                ast_file.traits.push(trait_decl);
            }
            TokenKind::ImplKeyword => {
                let impl_decl = self.parse_impl(annotations)?;
                ast_file.impls.push(impl_decl);
            }
            TokenKind::GivenKeyword => {
                ast_file.givens.push(self.parse_given(annotations)?);
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
