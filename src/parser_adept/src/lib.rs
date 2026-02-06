use document::{Document, DocumentRange};
use std::sync::Arc;
use syntax_tree::{BareSyntaxKind, BareSyntaxNode, SyntaxNode};
use text_edit::{LineIndex, TextLengthUtf16, TextPointUtf16};
use token::{Directive, Punct, Token, TokenKind};
use util_infinite_iterator::Peekable;
use util_text::{Character, CharacterPeeker};

pub struct Parser<II: Peekable<Token<()>>> {
    lexer: II,
}

impl<II> Parser<II>
where
    II: Peekable<Token<()>>,
{
    pub fn new(lexer: II) -> Self {
        Self { lexer }
    }

    pub fn run(&mut self) -> Arc<BareSyntaxNode> {
        let mut children = Vec::new();
        self.parse_all_whitespace(&mut children);

        while !self.lexer.peek().is_end_of_file() {
            self.parse_all_whitespace(&mut children);
            children.push(self.parse_top_level());
        }

        BareSyntaxNode::new_parent(BareSyntaxKind::Root, children)
    }

    fn parse_top_level(&mut self) -> Arc<BareSyntaxNode> {
        if self.should_parse_binding() {
            return self.parse_binding();
        }

        BareSyntaxNode::new_leaf(
            BareSyntaxKind::Error {
                description: "Expected top-level binding or attribute".into(),
            },
            self.lexer.next().kind.to_string(),
        )
    }

    fn should_parse_binding(&mut self) -> bool {
        if !self.lexer.peek().is_identifier() {
            return false;
        }

        let (after, _) = self
            .lexer
            .peek_skipping(1, |token| token.kind.is_column_spacing());

        if !after.kind.is_punct_of(Punct::new("::")) {
            return false;
        }

        true
    }

    fn error_for_next_token(&mut self, description: impl Into<String>) -> Arc<BareSyntaxNode> {
        let token = self.lexer.next();
        BareSyntaxNode::new_error(token.kind.to_string(), description.into())
    }

    fn parse_binding(&mut self) -> Arc<BareSyntaxNode> {
        if !self.lexer.peek().is_identifier() {
            return self.error_for_next_token("Expected identifier for binding");
        }

        let (after, _) = self
            .lexer
            .peek_skipping(1, |token| token.kind.is_column_spacing());

        if !after.kind.is_punct_of(Punct::new("::")) {
            return self.error_for_next_token("Expected `::` after identifier for binding");
        }

        let mut children = Vec::new();

        let name = self
            .lexer
            .eat(|token| match token.kind {
                TokenKind::Identifier(name) => Ok(name),
                _ => Err(token),
            })
            .unwrap();

        children.push(BareSyntaxNode::new_leaf(
            BareSyntaxKind::Identifier(name.clone().into()),
            name,
        ));

        self.parse_column_whitespace(&mut children);

        children.push(
            self.lexer
                .eat(|token| match token.kind {
                    TokenKind::Punct(punct) => {
                        assert!(punct == Punct::new("::"));
                        Ok(BareSyntaxNode::new_punct(punct))
                    }
                    _ => Err(token),
                })
                .unwrap(),
        );

        self.parse_column_whitespace(&mut children);
        children.push(self.parse_expr());

        BareSyntaxNode::new_parent(BareSyntaxKind::Binding, children)
    }

    fn parse_expr(&mut self) -> Arc<BareSyntaxNode> {
        if let Some(directive) = self.lexer.eat(|token| match token.kind {
            TokenKind::Directive(directive) => Ok(directive),
            _ => Err(token),
        }) {
            return self.parse_directive(directive);
        }

        self.error_for_next_token("Expected expression")
    }

    fn parse_directive(&mut self, directive: Directive) -> Arc<BareSyntaxNode> {
        match &directive {
            Directive::Standard("fn") => self.parse_fn_directive(),
            Directive::Standard(name) => BareSyntaxNode::new_error(
                directive.to_string(),
                format!("Directive `{}` is not supported yet", name),
            ),
            Directive::Unknown(name) => BareSyntaxNode::new_error(
                directive.to_string(),
                format!("Unknown directive `{}`", name),
            ),
        }
    }

    fn parse_fn_directive(&mut self) -> Arc<BareSyntaxNode> {
        if self
            .lexer
            .eat(|token| match token.kind {
                TokenKind::Punct(punct) if punct == Punct::new("(") => Ok(()),
                _ => Err(token),
            })
            .is_none()
        {
            return self.error_for_next_token("Expected `(` after `@fn` directive");
        }

        self.error_for_next_token("NOT SUPPORTED YET")
    }

    fn parse_column_whitespace(&mut self, children: &mut Vec<Arc<BareSyntaxNode>>) {
        while let Some(child) = self.lexer.eat(|token| match token.kind {
            TokenKind::ColumnSpacing(atom) => Ok(BareSyntaxNode::new_leaf(
                BareSyntaxKind::ColumnSpacing(atom),
                atom.to_string(),
            )),
            _ => Err(token),
        }) {
            children.push(child);
        }
    }

    fn parse_all_whitespace(&mut self, children: &mut Vec<Arc<BareSyntaxNode>>) {
        while let Some(child) = self.lexer.eat(|token| match token.kind {
            TokenKind::ColumnSpacing(atom) => Ok(BareSyntaxNode::new_leaf(
                BareSyntaxKind::ColumnSpacing(atom),
                atom.to_string(),
            )),
            TokenKind::LineSpacing(atom) => Ok(BareSyntaxNode::new_leaf(
                BareSyntaxKind::LineSpacing(atom),
                atom.to_string(),
            )),
            _ => Err(token),
        }) {
            children.push(child);
        }
    }
}

pub fn reparse(
    document: &Document,
    existing: Option<Arc<SyntaxNode>>,
    range: DocumentRange,
) -> Arc<SyntaxNode> {
    let adapter = util_infinite_iterator::Adapter::new(
        document.chars().map(|c| Character::At(c, ())),
        Character::End(()),
    );

    let lexer =
        util_infinite_iterator::Peeker::new(lexer_adept::Lexer::new(CharacterPeeker::new(adapter)));

    assert!(existing.is_none());
    let mut parser = Parser::new(lexer);

    SyntaxNode::new(
        None,
        parser.run(),
        TextPointUtf16 {
            line: LineIndex(0),
            col: TextLengthUtf16(0),
        },
    )
}

#[test]
fn test1() {
    use util_infinite_iterator::AsIter;

    let document = Document::new(r#"main :: @fn() {}"#.into());

    let adapter = util_infinite_iterator::Adapter::new(
        document.chars().map(|c| Character::At(c, ())),
        Character::End(()),
    );

    let mut lexer = lexer_adept::Lexer::new(CharacterPeeker::new(adapter));

    for item in lexer.as_iter(true) {
        println!("has {:?}", item);
    }

    let syntax_tree = reparse(&document, None, document.full_range());
    println!("{:#?}", syntax_tree);
}
