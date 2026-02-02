use document::{Document, DocumentRange};
use std::sync::Arc;
use syntax_tree::{BareSyntaxKind, BareSyntaxNode, SyntaxNode};
use text_edit::{LineIndex, TextLengthUtf16, TextPointUtf16};
use token::{Token, TokenKind};
use util_infinite_iterator::Peekable;
use util_text::{Character, CharacterPeeker};

pub enum State {
    Root(Vec<Arc<BareSyntaxNode>>),
    Done,
}

pub struct Parser<II: Peekable<Token<()>>> {
    stack: Vec<State>,
    lexer: II,
}

impl<II> Parser<II>
where
    II: Peekable<Token<()>>,
{
    pub fn new(lexer: II) -> Self {
        Self {
            stack: vec![State::Root(vec![])],
            lexer,
        }
    }

    pub fn run(&mut self) -> Arc<BareSyntaxNode> {
        loop {
            let state = self.stack.pop().unwrap();

            match state {
                State::Root(mut children) => match self.parse_root() {
                    Ok(child) => {
                        children.push(child);
                        self.stack.push(State::Root(children))
                    }
                    Err(new_state) => {
                        self.stack.push(State::Root(children));
                        self.stack.push(new_state);
                    }
                },
                State::Done => {
                    let Some(State::Root(children)) = self.stack.pop() else {
                        panic!("Invalid parse state");
                    };
                    return BareSyntaxNode::new_parent(BareSyntaxKind::Root, children);
                }
            }
        }
    }

    fn parse_root(&mut self) -> Result<Arc<BareSyntaxNode>, State> {
        if let Some(whitespace) = self.parse_whitespace() {
            return Ok(whitespace);
        }

        if self.lexer.peek().kind.is_end_of_file() {
            return Err(State::Done);
        }

        let Some(name) = self.lexer.eat(|token| match token.kind {
            TokenKind::Identifier(name) => Ok(name),
            _ => Err(token),
        }) else {
            return Ok(BareSyntaxNode::new_leaf(
                BareSyntaxKind::Error,
                self.lexer.next().kind.to_string(),
            ));
        };

        Ok(BareSyntaxNode::new_leaf(
            BareSyntaxKind::Identifier(name.clone().into()),
            name.into(),
        ))
    }

    fn parse_whitespace(&mut self) -> Option<Arc<BareSyntaxNode>> {
        self.lexer.eat(|token| match token.kind {
            TokenKind::ColumnSpacing(atom) => Ok(BareSyntaxNode::new_leaf(
                BareSyntaxKind::ColumnSpacing(atom),
                atom.to_string(),
            )),
            TokenKind::LineSpacing(atom) => Ok(BareSyntaxNode::new_leaf(
                BareSyntaxKind::LineSpacing(atom),
                atom.to_string(),
            )),
            _ => Err(token),
        })
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

    let document = Document::new(
        r#"
        main :: @fn
    "#
        .into(),
    );

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
