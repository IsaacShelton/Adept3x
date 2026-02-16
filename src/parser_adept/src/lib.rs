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

    fn error_for_empty(&mut self, description: impl Display) -> Arc<BareSyntaxNode> {
        BareSyntaxNode::new_error("".into(), description.to_string())
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
                variable_name => Ok(BareSyntaxNode::new_leaf(
                    BareSyntaxKind::Variable(variable_name.into()),
                    name.into(),
                )),
            },
            _ => Err(token),
        }) {
            return node;
        }

        if self.lexer.peek().is_punct_of_or_eof(Punct::new("}")) {
            self.error_for_empty("Expected expression")
        } else {
            self.error_for_next_token("Expected expression")
        }
    }

    fn parse_directive(&mut self, directive: Directive) -> Arc<BareSyntaxNode> {
        match &directive {
            Directive::Standard(name) => match name.as_ref() {
                "fn" => self.parse_fn_directive(directive),
                "if" => self.parse_if_directive(directive),
                _ => BareSyntaxNode::new_error(
                    directive.to_string(),
                    format!("Directive `{}` is not supported yet", name),
                ),
            },
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

    fn parse_fn_directive(&mut self, directive: Directive) -> Arc<BareSyntaxNode> {
        let mut children = Vec::new();
        children.push(BareSyntaxNode::new_leaf(
            BareSyntaxKind::Directive(directive.clone()),
            directive.to_string(),
        ));
        self.parse_column_whitespace(&mut children);
        children.push(self.parse_param_list());
        self.parse_column_whitespace(&mut children);
        self.parse_punct(Punct::new(":"), &mut children);
        children.push(self.parse_term());
        self.parse_column_whitespace(&mut children);
        children.push(self.parse_block());
        BareSyntaxNode::new_parent(BareSyntaxKind::Fn, children)
    }

    fn parse_block(&mut self) -> Arc<BareSyntaxNode> {
        let mut children = Vec::new();
        self.parse_punct(Punct::new("{"), &mut children);
        self.parse_all_whitespace(&mut children);
        children.push(self.parse_term());
        self.parse_all_whitespace(&mut children);
        self.parse_punct(Punct::new("}"), &mut children);
        BareSyntaxNode::new_parent(BareSyntaxKind::Block, children)
    }

    fn parse_if_directive(&mut self, directive: Directive) -> Arc<BareSyntaxNode> {
        // Ternary (for short single-line)
        // @if(a >= b, a + b, a - b)

        // Traditional (for multi-line)
        // @if a >= b { a + b } else { a - b }
        // @if (a >= b) { a + b } else { a - b }
        // @if(a >= b){ a + b }else{ a - b }

        let mut children = Vec::new();
        children.push(BareSyntaxNode::new_leaf(
            BareSyntaxKind::Directive(directive.clone()),
            directive.to_string(),
        ));

        self.parse_column_whitespace(&mut children);

        let has_block_args = if self.lexer.peek().is_punct_of(Punct::new("(")) {
            let arg_list = self.parse_if_arg_list();

            let has_arg_comma = arg_list.children().any(|child| {
                matches!(
                    child.as_ref().kind(),
                    BareSyntaxKind::Punct(punct) if *punct == Punct::new(","),
                )
            });

            let has_an_arg = arg_list
                .children()
                .any(|child| matches!(child.as_ref().kind(), BareSyntaxKind::Term));

            children.push(arg_list);
            !has_arg_comma && has_an_arg
        } else {
            children.push(self.parse_term());
            true
        };

        if has_block_args {
            self.parse_all_whitespace(&mut children);
            children.push(self.parse_block());
            self.parse_all_whitespace(&mut children);

            children.push(
                if let Some(else_keyword) = self.lexer.eat(|token| match token.kind {
                    TokenKind::Identifier(ident) if ident == "else" => {
                        Ok(BareSyntaxNode::new_leaf(
                            BareSyntaxKind::Identifier(ident.clone().into()),
                            ident,
                        ))
                    }
                    _ => Err(token),
                }) {
                    else_keyword
                } else {
                    self.error_for_empty("Expected `else` after first block of if")
                },
            );
            self.parse_all_whitespace(&mut children);

            children.push(self.parse_block());
        }

        BareSyntaxNode::new_parent(BareSyntaxKind::If, children)
    }

    fn parse_if_arg_list(&mut self) -> Arc<BareSyntaxNode> {
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

            children.push(self.parse_term());
            self.parse_all_whitespace(&mut children);
        }

        self.parse_punct(Punct::new(")"), &mut children);
        BareSyntaxNode::new_parent(BareSyntaxKind::IfArgList, children)
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

    fn parse_whitespace(&mut self, allow_newlines: bool, children: &mut Vec<Arc<BareSyntaxNode>>) {
        while let Some(child) = self.lexer.eat(|token| match token.kind {
            TokenKind::ColumnSpacing(atom) => Ok(BareSyntaxNode::new_leaf(
                BareSyntaxKind::ColumnSpacing(atom),
                atom.to_string(),
            )),
            TokenKind::LineSpacing(atom) if allow_newlines => Ok(BareSyntaxNode::new_leaf(
                BareSyntaxKind::LineSpacing(atom),
                atom.to_string(),
            )),
            TokenKind::SinglelineComment(comment) => Ok(BareSyntaxNode::new_leaf(
                BareSyntaxKind::SinglelineComment(comment.clone().into()),
                comment,
            )),
            TokenKind::MultilineComment(comment, terminated) => Ok(BareSyntaxNode::new_leaf(
                BareSyntaxKind::MultilineComment(comment.clone().into(), terminated),
                comment,
            )),
            _ => Err(token),
        }) {
            children.push(child);
        }
    }

    fn parse_column_whitespace(&mut self, children: &mut Vec<Arc<BareSyntaxNode>>) {
        self.parse_whitespace(false, children);
    }

    fn parse_all_whitespace(&mut self, children: &mut Vec<Arc<BareSyntaxNode>>) {
        self.parse_whitespace(true, children);
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
    let document = Document::new(
        r#"
        test1 :: @fn(): Bool {}

        test2 :: @fn(): Void {}

        test3 :: @fn(a: Bool): Void {}

        test4 :: @fn(a: Bool, b: Bool): Bool {}
        
        test5 :: @fn(a: Bool, b: Bool, c: Bool): Bool {}

        test6 :: @fn(a, b, c, d: Bool): Bool {}

        test8 :: @fn(): Bool { true }

        test9 :: @fn(x: Bool): Type {
            @if(x, Bool, Void)
        }

        test_if_else :: @fn(a, b: Bool): @if(a, Bool, Void) {}

        test10 :: @fn(a, b: Bool): @if a { Bool } else { Void } {}

        test11 :: @fn(a, b: Bool): @if(a){ Bool } else { Void } {}

        test12 :: @fn(a, b: Bool): @if (a) { Bool } else { Void } {}

        /*
        Test13 :: @Record {
            a: Bool,
            b: Bool,
            c: Void,
            d: Type,
        }

        test_if_else :: @fn(a, b: Bool): if a { Bool } else { Void } {}

        //my_pair :: @record { a: true, b: false }

        make_pair :: @fn(T: Type, a, b: T): Pair(T) {
            @record { a, b }
        }

        Pair :: @fn(T: Type): Type {
            @Record {
                a: T,
                b: T,
            }
        }
        */
        "#
        .into(),
    );

    /*
    let adapter = util_infinite_iterator::Adapter::new(
        document.chars().map(|c| Character::At(c, ())),
        Character::End(()),
    );

    let mut lexer = lexer_adept::Lexer::new(CharacterPeeker::new(adapter));

    for item in lexer.as_iter(true) {
        println!("has {:?}", item);
    }
    */

    let syntax_tree = reparse(&document, None, document.full_range());
    let _ = syntax_tree.dump(&mut std::io::stdout(), 0);
}
