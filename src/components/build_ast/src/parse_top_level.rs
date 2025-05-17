use super::{
    Parser,
    annotation::{Annotation, AnnotationKind},
    error::{ParseError, ParseErrorKind},
};
use ast::NamespaceItems;
use infinite_iterator::InfinitePeekable;
use token::{Token, TokenKind};

impl<'a, I: InfinitePeekable<Token>> Parser<'a, I> {
    pub fn parse_namespace_items(&mut self) -> Result<NamespaceItems, ParseError> {
        let mut items = NamespaceItems::default();
        while !self.input.peek_is_or_eof(TokenKind::CloseCurly) {
            self.parse_top_level(&mut items, vec![])?;
            self.ignore_newlines();
        }
        Ok(items)
    }

    pub fn parse_top_level(
        &mut self,
        namespace_items: &mut NamespaceItems,
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
                    self.parse_top_level(namespace_items, annotations.clone())?;
                    self.ignore_newlines();
                }

                self.input
                    .expect(TokenKind::CloseCurly, "to close annotation group")?;
            }
            TokenKind::PragmaKeyword => {
                return Err(ParseErrorKind::Other {
                    message:
                        "Cannot use 'pragma' keyword here, did you mean to compile as single file?"
                            .into(),
                }
                .at(self.input.peek().source));
            }
            TokenKind::FuncKeyword => {
                namespace_items.funcs.push(self.parse_func(annotations)?);
            }
            TokenKind::Identifier(_) => {
                namespace_items
                    .globals
                    .push(self.parse_global(annotations)?);
            }
            TokenKind::StructKeyword => namespace_items
                .structs
                .push(self.parse_structure(annotations)?),
            TokenKind::TypeAliasKeyword => {
                let type_alias = self.parse_type_alias(annotations)?;
                namespace_items.type_aliases.push(type_alias);
            }
            TokenKind::EnumKeyword => {
                let enum_definition = self.parse_enum(annotations)?;

                namespace_items.enums.push(enum_definition);
            }
            TokenKind::DefineKeyword => {
                let helper_expr = self.parse_helper_expr(annotations)?;
                namespace_items.expr_aliases.push(helper_expr);
            }
            TokenKind::TraitKeyword => {
                let trait_decl = self.parse_trait(annotations)?;
                namespace_items.traits.push(trait_decl);
            }
            TokenKind::ImplKeyword => {
                namespace_items.impls.push(self.parse_impl(annotations)?);
            }
            TokenKind::NamespaceKeyword => {
                namespace_items
                    .namespaces
                    .push(self.parse_namespace(annotations)?);
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
