use document::{Document, DocumentRange};
use std::sync::Arc;
use syntax_tree::{BareSyntaxKind, BareSyntaxNode, SyntaxNode};
use text_edit::{LineIndex, TextLengthUtf16, TextPointUtf16};
use util_text::{Character, CharacterPeeker};

pub enum State {
    Root,
}

// NOTE: Parsing never fails. Errors are allowed within syntax.
pub fn reparse(
    document: &Document,
    existing: Option<Arc<SyntaxNode>>,
    range: DocumentRange,
) -> Arc<SyntaxNode> {
    eprintln!("reparse is unimplemented!");

    let bare_syntax_node = BareSyntaxNode::new_leaf(BareSyntaxKind::Null, "".into());

    let adapter = util_infinite_iterator::Adapter::new(
        document.chars().map(|c| Character::At(c, ())),
        Character::End(()),
    );

    let _lexer = lexer_adept::Lexer::new(CharacterPeeker::new(adapter));

    SyntaxNode::new(
        None,
        bare_syntax_node,
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
    main :: @fn() {
        io.println("Hello, world!, and his name is \"John\".")
    }

    String :: @type {
        bytes: Ptr'U8,
        length: USize,
        capacity: USize,
    }

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
}

/*
use parser_abstract::Parser;
use util_infinite_iterator::InfiniteIterator;

struct AdeptParser {}

enum AdeptParserState {
    Root,
}

impl Parser for AdeptParser {
    type State = AdeptParserState;
    type Token = ();
    type Output = ();

    fn parse(
        &self,
        starting_state: Option<Self::State>,
        tokens: impl InfiniteIterator<Item = Self::Token>,
    ) -> Self::Output {
        let state = starting_state.unwrap_or(AdeptParserState::Root);
        todo!("AdeptParser::parse")
    }
}
*/
