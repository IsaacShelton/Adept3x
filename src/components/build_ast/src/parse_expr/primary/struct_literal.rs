use super::{ParseError, Parser};
use ast::{Expr, ExprKind, FieldInitializer, FillBehavior, Language, Name, StructLiteral, Type};
use infinite_iterator::InfinitePeekable;
use token::{Token, TokenKind};

impl<'a, I: InfinitePeekable<Token>> Parser<'a, I> {
    pub fn parse_struct_literal(&mut self) -> Result<Expr, ParseError> {
        // Type { x: VALUE, b: VALUE, c: VALUE, :d, :e, ..SPECIFIER }
        //  ^

        let ast_type = self.parse_type(None::<&str>, Some("for type of struct literal"))?;
        self.parse_struct_literal_with(ast_type)
    }

    pub fn parse_struct_literal_with(&mut self, ast_type: Type) -> Result<Expr, ParseError> {
        // Type { x: VALUE, b: VALUE, c: VALUE, :d, :e, ..SPECIFIER }
        //      ^

        let source = ast_type.source;
        self.parse_token(TokenKind::OpenCurly, Some("to begin struct literal"))?;
        self.ignore_newlines();

        let mut fill_behavior = FillBehavior::Forbid;
        let mut fields = Vec::new();

        while !self.input.peek_is_or_eof(TokenKind::CloseCurly) {
            if self.input.eat(TokenKind::Extend) {
                if self.input.eat(TokenKind::ZeroedKeyword) {
                    fill_behavior = FillBehavior::Zeroed;
                }
            } else {
                let dupe = self.input.eat(TokenKind::Colon);
                let field_name = self.parse_identifier(Some("for field name in struct literal"))?;

                self.ignore_newlines();

                let field_value = if dupe {
                    ExprKind::Variable(Name::plain(field_name.clone())).at(source)
                } else {
                    self.parse_token(TokenKind::Colon, Some("after field name in struct literal"))?;
                    self.ignore_newlines();
                    let value = self.parse_expr()?;
                    self.ignore_newlines();
                    value
                };

                fields.push(FieldInitializer {
                    name: Some(field_name),
                    value: field_value,
                });
            }

            self.ignore_newlines();
            if !self.input.peek_is(TokenKind::CloseCurly) {
                self.parse_token(TokenKind::Comma, Some("after field in struct literal"))?;
                self.ignore_newlines();
            }
        }

        self.parse_token(TokenKind::CloseCurly, Some("to end struct literal"))?;
        Ok(Expr::new(
            ExprKind::StructLiteral(Box::new(StructLiteral {
                ast_type,
                fields,
                fill_behavior,
                language: Language::Adept,
            })),
            source,
        ))
    }
}
