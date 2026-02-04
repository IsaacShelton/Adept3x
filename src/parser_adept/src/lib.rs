use document::{Document, DocumentRange};
use std::sync::Arc;
use syntax_tree::{BareSyntaxKind, BareSyntaxNode, SyntaxNode};
use text_edit::{LineIndex, TextLengthUtf16, TextPointUtf16};
use token::{Punct, Token, TokenKind};
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
        if let Some(binding) = self.parse_binding() {
            return binding;
        }

        BareSyntaxNode::new_leaf(BareSyntaxKind::Error, self.lexer.next().kind.to_string())
    }

    fn parse_binding(&mut self) -> Option<Arc<BareSyntaxNode>> {
        if !self.lexer.peek().is_identifier() {
            return None;
        }

        let (after, _) = self
            .lexer
            .peek_skipping(1, |token| token.kind.is_column_spacing());

        if !after.kind.is_punct_of(Punct::new("::")) {
            return None;
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

        Some(BareSyntaxNode::new_parent(
            BareSyntaxKind::Binding,
            children,
        ))
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
