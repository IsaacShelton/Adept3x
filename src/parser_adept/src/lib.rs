use document::{Document, DocumentRange};
use lazy_format::lazy_format;
use std::{fmt::Display, sync::Arc};
use syntax_tree::{BareSyntaxKind, BareSyntaxNode, BuiltinType, BuiltinValue, SyntaxNode};
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
            children.push(self.parse_top_level());
            self.parse_all_whitespace(&mut children);
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

    fn error_for_next_token(&mut self, description: impl Display) -> Arc<BareSyntaxNode> {
        let token = self.lexer.next();
        BareSyntaxNode::new_error(token.kind.to_string(), description.to_string())
    }

    fn parse_name(&mut self) -> Arc<BareSyntaxNode> {
        if let Some(ok) = self.lexer.eat(|token| match token.kind {
            TokenKind::Identifier(name) => {
                let identifier =
                    BareSyntaxNode::new_leaf(BareSyntaxKind::Identifier(name.clone().into()), name);

                Ok(BareSyntaxNode::new_parent(
                    BareSyntaxKind::Name,
                    vec![identifier],
                ))
            }
            _ => Err(token),
        }) {
            ok
        } else {
            self.error_for_next_token("Expected name")
        }
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
        children.push(self.parse_name());
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
        children.push(self.parse_term());

        BareSyntaxNode::new_parent(BareSyntaxKind::Binding, children)
    }

    fn parse_term(&mut self) -> Arc<BareSyntaxNode> {
        let mut children = vec![];
        self.parse_column_whitespace(&mut children);
        children.push(self.parse_term_inner());
        BareSyntaxNode::new_parent(BareSyntaxKind::Term, children)
    }

    fn parse_term_inner(&mut self) -> Arc<BareSyntaxNode> {
        if let Some(directive) = self.lexer.eat(|token| match token.kind {
            TokenKind::Directive(directive) => Ok(directive),
            _ => Err(token),
        }) {
            return self.parse_directive(directive);
        }

        if let Some(node) = self.lexer.eat(|token| match &token.kind {
            TokenKind::Identifier(name) => match name.as_str() {
                "Bool" => Ok(BareSyntaxNode::new_leaf(
                    BareSyntaxKind::BuiltinType(BuiltinType::Bool),
                    name.into(),
                )),
                "Void" => Ok(BareSyntaxNode::new_leaf(
                    BareSyntaxKind::BuiltinType(BuiltinType::Void),
                    name.into(),
                )),
                "Type" => Ok(BareSyntaxNode::new_leaf(
                    BareSyntaxKind::BuiltinType(BuiltinType::Type),
                    name.into(),
                )),
                "true" => Ok(BareSyntaxNode::new_leaf(
                    BareSyntaxKind::BuiltinValue(BuiltinValue::True),
                    name.into(),
                )),
                "false" => Ok(BareSyntaxNode::new_leaf(
                    BareSyntaxKind::BuiltinValue(BuiltinValue::False),
                    name.into(),
                )),
                "void" => Ok(BareSyntaxNode::new_leaf(
                    BareSyntaxKind::BuiltinValue(BuiltinValue::Void),
                    name.into(),
                )),
                _ => Err(token),
            },
            _ => Err(token),
        }) {
            return node;
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

    fn parse_punct(&mut self, expected: Punct, children: &mut Vec<Arc<BareSyntaxNode>>) {
        let result = if let Some(result) = self.lexer.eat(|token| match token.kind {
            TokenKind::Punct(punct) if punct == expected => Ok(BareSyntaxNode::new_leaf(
                BareSyntaxKind::Punct(punct),
                punct.to_string(),
            )),
            _ => Err(token),
        }) {
            result
        } else {
            self.error_for_next_token(lazy_format!("Expected `{}`", expected))
        };

        children.push(result);
    }

    fn parse_fn_directive(&mut self) -> Arc<BareSyntaxNode> {
        let mut children = Vec::new();
        self.parse_column_whitespace(&mut children);
        children.push(self.parse_param_list());
        self.parse_column_whitespace(&mut children);
        self.parse_punct(Punct::new(":"), &mut children);
        children.push(self.parse_term());
        self.parse_column_whitespace(&mut children);
        self.parse_punct(Punct::new("{"), &mut children);
        self.parse_column_whitespace(&mut children);
        self.parse_punct(Punct::new("}"), &mut children);
        BareSyntaxNode::new_parent(BareSyntaxKind::Fn, children)
    }

    fn parse_param_list(&mut self) -> Arc<BareSyntaxNode> {
        let mut children = vec![];
        self.parse_punct(Punct::new("("), &mut children);
        self.parse_all_whitespace(&mut children);

        let mut has_param = false;

        while !self.lexer.peek().kind.is_punct_of_or_eof(Punct::new(")")) {
            if has_param {
                self.parse_punct(Punct::new(","), &mut children);
                self.parse_all_whitespace(&mut children);
            } else {
                has_param = true;
            }

            children.push(self.parse_param_sublist());
            self.parse_all_whitespace(&mut children);
        }

        self.parse_punct(Punct::new(")"), &mut children);
        BareSyntaxNode::new_parent(BareSyntaxKind::ParamList, children)
    }

    fn parse_param_sublist(&mut self) -> Arc<BareSyntaxNode> {
        let mut children = vec![];
        self.parse_param_names(&mut children);
        self.parse_punct(Punct::new(":"), &mut children);
        children.push(self.parse_term());
        BareSyntaxNode::new_parent(BareSyntaxKind::Param, children)
    }

    fn parse_param_names(&mut self, children: &mut Vec<Arc<BareSyntaxNode>>) {
        children.push(self.parse_name());
        self.parse_all_whitespace(children);

        while self.lexer.peek().is_punct_of(Punct::new(",")) {
            self.parse_punct(Punct::new(","), children);
            self.parse_all_whitespace(children);
            children.push(self.parse_name());
            self.parse_all_whitespace(children);
        }
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

    let document = Document::new(
        r#"
        test1 :: @fn(): Bool {}

        test2 :: @fn(): Void {}

        test3 :: @fn(a: Bool): Void {}

        test4 :: @fn(a: Bool, b: Bool): Bool {}
        
        test5 :: @fn(a: Bool, b: Bool, c: Bool): Bool {}

        sum :: @fn(a, b, c, d: Bool): Bool {}
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
    let _ = syntax_tree.dump(&mut std::io::stdout(), 0);
}
