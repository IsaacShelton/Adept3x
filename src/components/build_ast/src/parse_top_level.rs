use super::{
    Parser,
    annotation::{Annotation, AnnotationKind},
    error::ParseError,
};
use ast::RawAstFile;
use infinite_iterator::InfinitePeekable;
use token::{Token, TokenKind};

impl<'a, I: InfinitePeekable<Token>> Parser<'a, I> {
    pub fn parse_top_level(
        &mut self,
        ast_file: &mut RawAstFile,
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
                ast_file.funcs.push(self.parse_func(annotations)?);
            }
            TokenKind::Identifier(_) => {
                ast_file.globals.push(self.parse_global(annotations)?);
            }
            TokenKind::StructKeyword => ast_file.structs.push(self.parse_structure(annotations)?),
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
                ast_file.expr_aliases.push(helper_expr);
            }
            TokenKind::TraitKeyword => {
                let trait_decl = self.parse_trait(annotations)?;
                ast_file.traits.push(trait_decl);
            }
            TokenKind::ImplKeyword => {
                ast_file.impls.push(self.parse_impl(annotations)?);
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
