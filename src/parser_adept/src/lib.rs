use document::{Document, DocumentRange};
use lazy_format::lazy_format;
use std::{fmt::Display, sync::Arc};
use syntax_tree::{BareSyntaxKind, BareSyntaxNode, BuiltinType, Reparsable, SyntaxNode};
use text_edit::{LineIndex, TextLengthUtf16, TextPointUtf16};
use token::{Directive, Punct, Token, TokenKind};
use util_infinite_iterator::Peekable;
use util_text::{Character, CharacterPeeker, LineSpacingAtom};

pub struct Parser<II: Peekable<Token<()>>> {
    lexer: II,
}

pub enum ErrorRecovery {
    Empty,
    EatOne,
    EatUntilNestedClosing(TokenKind),
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
                description: "Expected top-level binding".into(),
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

    fn error_for_empty(description: impl Display) -> Arc<BareSyntaxNode> {
        BareSyntaxNode::new_error("".into(), description.to_string())
    }

    fn error_for_next_token(&mut self, description: impl Display) -> Arc<BareSyntaxNode> {
        let token = self.lexer.next();
        BareSyntaxNode::new_error(token.kind.to_string(), description.to_string())
    }

    fn error_until(
        &mut self,
        closing_token: TokenKind,
        description: impl Display,
    ) -> Arc<BareSyntaxNode> {
        self.error_until_any(&[closing_token], description)
    }

    fn error_until_any(
        &mut self,
        closing_tokens: &[TokenKind],
        description: impl Display,
    ) -> Arc<BareSyntaxNode> {
        let mut raw_content = String::new();

        while self
            .lexer
            .eat(|token| {
                if closing_tokens.contains(&token.kind) || token.is_end_of_file() {
                    Err(token)
                } else {
                    raw_content.push_str(&token.to_string());
                    Ok(())
                }
            })
            .is_some()
        {}

        BareSyntaxNode::new_error(raw_content, description.to_string())
    }

    fn parse_name_required(&mut self) -> Arc<BareSyntaxNode> {
        self.parse_name()
            .unwrap_or_else(|| self.error_for_next_token("Expected name"))
    }

    fn parse_name(&mut self) -> Option<Arc<BareSyntaxNode>> {
        self.lexer.eat(|token| match token.kind {
            TokenKind::Identifier(name) => {
                let identifier =
                    BareSyntaxNode::new_leaf(BareSyntaxKind::Identifier(name.clone().into()), name);

                Ok(BareSyntaxNode::new_parent(
                    BareSyntaxKind::Name,
                    vec![identifier],
                ))
            }
            _ => Err(token),
        })
    }

    fn parse_binding(&mut self) -> Arc<BareSyntaxNode> {
        let mut children = Vec::new();
        children.push(self.parse_name_required());
        self.parse_column_whitespace(&mut children);

        if self
            .parse_punct(Punct::new("::"), &mut children, ErrorRecovery::Empty)
            .is_ok()
        {
            self.parse_column_whitespace(&mut children);
            children.push(self.parse_term());

            if self.parse_all_whitespace(&mut children).is_none()
                && !self.lexer.peek().is_end_of_file()
            {
                children.push(Self::error_for_empty("Expected newline after binding"))
            }
        }

        BareSyntaxNode::new_parent(BareSyntaxKind::Binding, children)
    }

    fn parse_term(&mut self) -> Arc<BareSyntaxNode> {
        let mut top_children = vec![];
        self.parse_column_whitespace(&mut top_children);

        let mut term_inner = vec![self.parse_term_inner()];

        let term_children = loop {
            self.parse_column_whitespace(&mut term_inner);

            match self.parse_term_post(term_inner) {
                Ok(new_result) => term_inner = vec![new_result],
                Err(done) => break done,
            }
        };

        top_children.extend(term_children);
        BareSyntaxNode::new_parent(BareSyntaxKind::Term, top_children)
    }

    fn parse_term_post(
        &mut self,
        mut children: Vec<Arc<BareSyntaxNode>>,
    ) -> Result<Arc<BareSyntaxNode>, Vec<Arc<BareSyntaxNode>>> {
        if self.lexer.peek().is_punct_of(Punct::new("(")) {
            children = vec![
                BareSyntaxNode::new_parent(BareSyntaxKind::Term, children),
                self.parse_arg_list(Reparsable::Reparse),
            ];
            return Ok(BareSyntaxNode::new_parent(BareSyntaxKind::Call, children));
        }

        if self.lexer.peek().is_punct_of(Punct::new(":=")) {
            children = vec![
                BareSyntaxNode::new_parent(BareSyntaxKind::Term, children),
                BareSyntaxNode::new_punct(self.lexer.next().kind.unwrap_punct()),
                self.parse_term(),
            ];

            if self.parse_all_whitespace(&mut children).is_some() {
                children.push(self.parse_term());
            }

            return Ok(BareSyntaxNode::new_parent(BareSyntaxKind::Let, children));
        }

        if self.lexer.peek().is_punct_of(Punct::new(":")) {
            children = vec![
                BareSyntaxNode::new_parent(BareSyntaxKind::Term, children),
                BareSyntaxNode::new_punct(self.lexer.next().kind.unwrap_punct()),
            ];

            self.parse_column_whitespace(&mut children);

            if self.lexer.peek().is_punct_of(Punct::new("=")) {
                children.push(BareSyntaxNode::new_punct(
                    self.lexer.next().kind.unwrap_punct(),
                ));
                children.push(self.parse_term());
            } else {
                children.push(self.parse_term());

                if self
                    .parse_punct(Punct::new("="), &mut children, ErrorRecovery::Empty)
                    .is_ok()
                {
                    children.push(self.parse_term());
                }
            }

            if self.parse_all_whitespace(&mut children).is_some() {
                children.push(self.parse_term());
            }

            return Ok(BareSyntaxNode::new_parent(BareSyntaxKind::Let, children));
        }

        if self.lexer.peek().is_punct_of(Punct::new(".")) && self.lexer.peek_nth(1).is_integer() {
            children = vec![
                BareSyntaxNode::new_parent(BareSyntaxKind::Term, children),
                BareSyntaxNode::new_punct(self.lexer.next().kind.unwrap_punct()),
            ];
            let (integer, text) = self.lexer.next().kind.unwrap_integer();
            children.push(BareSyntaxNode::new_leaf(
                BareSyntaxKind::Integer(integer),
                text,
            ));

            return Ok(BareSyntaxNode::new_parent(BareSyntaxKind::Nth, children));
        }

        Err(children)
    }

    fn parse_term_inner(&mut self) -> Arc<BareSyntaxNode> {
        if let Some(directive) = self.lexer.eat(|token| match token.kind {
            TokenKind::Directive(directive) => Ok(directive),
            _ => Err(token),
        }) {
            return self.parse_directive(directive);
        }

        {
            let mut children = vec![];
            if self
                .parse_punct(Punct::new("("), &mut children, ErrorRecovery::Empty)
                .is_ok()
            {
                self.parse_all_whitespace(&mut children);
                children.push(self.parse_term());
                self.parse_all_whitespace(&mut children);
                let _ = self.parse_punct(Punct::new(")"), &mut children, ErrorRecovery::Empty);
                return BareSyntaxNode::new_parent(BareSyntaxKind::ParenthesizedTerm, children);
            }
        }

        if let Some(node) = self.lexer.eat(|token| match &token.kind {
            TokenKind::Integer(value, text) => Ok(BareSyntaxNode::new_leaf(
                BareSyntaxKind::Integer(Arc::clone(value)),
                text.into(),
            )),
            TokenKind::Identifier(name) => match name.as_str() {
                "Type" => Ok(BareSyntaxNode::new_leaf(
                    BareSyntaxKind::BuiltinType(BuiltinType::Type),
                    name.into(),
                )),
                "Void" => Ok(BareSyntaxNode::new_leaf(
                    BareSyntaxKind::BuiltinType(BuiltinType::Void),
                    name.into(),
                )),
                "Bool" => Ok(BareSyntaxNode::new_leaf(
                    BareSyntaxKind::BuiltinType(BuiltinType::Bool),
                    name.into(),
                )),
                "Nat" => Ok(BareSyntaxNode::new_leaf(
                    BareSyntaxKind::BuiltinType(BuiltinType::Nat),
                    name.into(),
                )),
                "true" => Ok(BareSyntaxNode::new_leaf(
                    BareSyntaxKind::TrueValue,
                    name.into(),
                )),
                "false" => Ok(BareSyntaxNode::new_leaf(
                    BareSyntaxKind::FalseValue,
                    name.into(),
                )),
                "void" => Ok(BareSyntaxNode::new_leaf(
                    BareSyntaxKind::VoidValue,
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

        if self.lexer.peek().is_punct_of(Punct::new(",")) {
            Self::error_for_empty("Expected expression before `,`")
        } else if self.lexer.peek().is_punct_of(Punct::new(")")) {
            Self::error_for_empty("Expected expression before `)`")
        } else if self.lexer.peek().is_punct_of(Punct::new("}")) {
            Self::error_for_empty("Expected expression before `}`")
        } else if self.lexer.peek().is_line_spacing() {
            Self::error_for_empty("Expected expression")
        } else if self.lexer.peek().is_end_of_file() {
            Self::error_for_empty("Expected expression before end-of-file")
        } else {
            self.error_for_next_token("Expected expression")
        }
    }

    fn parse_directive(&mut self, directive: Directive) -> Arc<BareSyntaxNode> {
        match &directive {
            Directive::Standard(name) => match name.as_ref() {
                "fn" => self.parse_fn_directive(directive),
                "if" => self.parse_if_directive(directive),
                "bool_elim" => self.parse_elim_directive(directive, BareSyntaxKind::BoolElim),
                "nat_elim" => self.parse_elim_directive(directive, BareSyntaxKind::NatElim),
                "nat_succ" => self.parse_intro_directive(directive, BareSyntaxKind::NatSucc),
                "match" => self.parse_match_directive(directive),
                "Fn" => self.parse_fn_type_directive(directive),
                "Record" => self.parse_record_type_directive(directive),
                "record" => self.parse_record_directive(directive),
                "eval" => self.parse_eval(directive),
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

    fn parse_punct(
        &mut self,
        expected: Punct,
        children: &mut Vec<Arc<BareSyntaxNode>>,
        error_recovery: ErrorRecovery,
    ) -> Result<(), ()> {
        if let Some(result) = self.lexer.eat(|token| match token.kind {
            TokenKind::Punct(punct) if punct == expected => Ok(BareSyntaxNode::new_leaf(
                BareSyntaxKind::Punct(punct),
                punct.to_string(),
            )),
            _ => Err(token),
        }) {
            children.push(result);
            Ok(())
        } else {
            match error_recovery {
                ErrorRecovery::Empty => {
                    children.push(Self::error_for_empty(lazy_format!(
                        "Expected `{}`",
                        expected
                    )));
                }
                ErrorRecovery::EatOne => {
                    children
                        .push(self.error_for_next_token(lazy_format!("Expected `{}`", expected)));
                }
                ErrorRecovery::EatUntilNestedClosing(token_kind) => {
                    children.push(
                        self.error_until(token_kind, lazy_format!("Expected `{}`", expected)),
                    );
                }
            }
            Err(())
        }
    }

    fn parse_fn_type_directive(&mut self, directive: Directive) -> Arc<BareSyntaxNode> {
        let mut children = Vec::new();
        children.push(BareSyntaxNode::new_leaf(
            BareSyntaxKind::Directive(directive.clone()),
            directive.to_string(),
        ));
        self.parse_column_whitespace(&mut children);
        match self.parse_param_list() {
            Ok(node) => {
                children.push(node);
                self.parse_column_whitespace(&mut children);
                self.parse_type_annotation(false, &mut children);
            }
            Err(node) => {
                children.push(node);
                self.parse_column_whitespace(&mut children);
            }
        }
        BareSyntaxNode::new_parent(BareSyntaxKind::BuiltinType(BuiltinType::Fn), children)
    }

    fn parse_record_type_directive(&mut self, directive: Directive) -> Arc<BareSyntaxNode> {
        let mut children = Vec::new();
        children.push(BareSyntaxNode::new_leaf(
            BareSyntaxKind::Directive(directive.clone()),
            directive.to_string(),
        ));
        self.parse_column_whitespace(&mut children);
        children.push(self.parse_field_def_list());
        BareSyntaxNode::new_parent(BareSyntaxKind::BuiltinType(BuiltinType::Record), children)
    }

    fn parse_eval(&mut self, directive: Directive) -> Arc<BareSyntaxNode> {
        let mut children = Vec::new();
        children.push(BareSyntaxNode::new_leaf(
            BareSyntaxKind::Directive(directive.clone()),
            directive.to_string(),
        ));
        children.push(self.parse_term());
        BareSyntaxNode::new_parent(BareSyntaxKind::Eval, children)
    }

    fn parse_fn_directive(&mut self, directive: Directive) -> Arc<BareSyntaxNode> {
        let mut children = Vec::new();
        children.push(BareSyntaxNode::new_leaf(
            BareSyntaxKind::Directive(directive.clone()),
            directive.to_string(),
        ));
        self.parse_column_whitespace(&mut children);

        match self.parse_param_list() {
            Ok(node) => {
                children.push(node);
                self.parse_column_whitespace(&mut children);
                self.parse_type_annotation(false, &mut children);
                self.parse_column_whitespace(&mut children);
                children.push(self.parse_block());
            }
            Err(node) => {
                children.push(node);
                self.parse_column_whitespace(&mut children);
            }
        }

        BareSyntaxNode::new_parent(BareSyntaxKind::FnValue, children)
    }

    fn parse_block(&mut self) -> Arc<BareSyntaxNode> {
        let mut children = Vec::new();
        if self
            .parse_punct(Punct::new("{"), &mut children, ErrorRecovery::Empty)
            .is_ok()
        {
            self.parse_all_whitespace(&mut children);
            children.push(self.parse_term());
            self.parse_all_whitespace(&mut children);

            let _ = self.parse_punct(
                Punct::new("}"),
                &mut children,
                // TODO: This should take ideally nesting into account
                ErrorRecovery::EatUntilNestedClosing(TokenKind::Punct(Punct::new("}"))),
            );
        }

        BareSyntaxNode::new_parent(BareSyntaxKind::Block, children)
    }

    fn parse_match_directive(&mut self, directive: Directive) -> Arc<BareSyntaxNode> {
        /*
        @match x @so(x) P(x) {
            true => 0,
            false => 1,
        }
        */

        let mut children = Vec::new();
        children.push(BareSyntaxNode::new_leaf(
            BareSyntaxKind::Directive(directive.clone()),
            directive.to_string(),
        ));

        self.parse_column_whitespace(&mut children);
        children.push(self.parse_term());
        children.push(self.parse_match_block());
        BareSyntaxNode::new_parent(BareSyntaxKind::Match, children)
    }

    fn parse_match_block(&mut self) -> Arc<BareSyntaxNode> {
        let mut children = Vec::new();
        if self
            .parse_punct(Punct::new("{"), &mut children, ErrorRecovery::Empty)
            .is_ok()
        {
            self.parse_all_whitespace(&mut children);
            children.push(self.parse_match_arm());
            self.parse_all_whitespace(&mut children);

            let _ = self.parse_punct(
                Punct::new("}"),
                &mut children,
                // TODO: This should take ideally nesting into account
                ErrorRecovery::EatUntilNestedClosing(TokenKind::Punct(Punct::new("}"))),
            );
        }

        BareSyntaxNode::new_parent(BareSyntaxKind::MatchBlock, children)
    }

    fn parse_match_arm(&mut self) -> Arc<BareSyntaxNode> {
        let mut children = Vec::new();
        children.push(self.parse_pattern());
        let _ = self.parse_punct(Punct::new("=>"), &mut children, ErrorRecovery::Empty);
        children.push(self.parse_term());

        BareSyntaxNode::new_parent(BareSyntaxKind::MatchArm, children)
    }

    fn parse_pattern(&mut self) -> Arc<BareSyntaxNode> {
        let mut children = Vec::new();

        if let Some(node) = self.lexer.eat(|token| match &token.kind {
            TokenKind::Integer(value, text) => Ok(BareSyntaxNode::new_leaf(
                BareSyntaxKind::Integer(Arc::clone(value)),
                text.into(),
            )),
            TokenKind::Identifier(name) => match name.as_str() {
                "true" => Ok(BareSyntaxNode::new_leaf(
                    BareSyntaxKind::TrueValue,
                    name.into(),
                )),
                "false" => Ok(BareSyntaxNode::new_leaf(
                    BareSyntaxKind::FalseValue,
                    name.into(),
                )),
                "void" => Ok(BareSyntaxNode::new_leaf(
                    BareSyntaxKind::VoidValue,
                    name.into(),
                )),
                _ => Err(token),
            },
            _ => Err(token),
        }) {
            children.push(node);
        }

        BareSyntaxNode::new_parent(BareSyntaxKind::Pattern, children)
    }

    fn parse_intro_directive(
        &mut self,
        directive: Directive,
        kind: BareSyntaxKind,
    ) -> Arc<BareSyntaxNode> {
        // (same as elim directive)
        self.parse_elim_directive(directive, kind)
    }

    fn parse_elim_directive(
        &mut self,
        directive: Directive,
        kind: BareSyntaxKind,
    ) -> Arc<BareSyntaxNode> {
        let mut children = Vec::new();
        children.push(BareSyntaxNode::new_leaf(
            BareSyntaxKind::Directive(directive.clone()),
            directive.to_string(),
        ));

        self.parse_column_whitespace(&mut children);
        let arg_list = self.parse_arg_list(Reparsable::Reparse);
        children.push(arg_list);
        self.parse_column_whitespace(&mut children);

        BareSyntaxNode::new_parent(kind, children)
    }

    fn parse_if_directive(&mut self, directive: Directive) -> Arc<BareSyntaxNode> {
        // Ternary (for short single-line)
        // @if(a >= b, a + b, a - b)

        // Ternary with Motive (for dependent if)
        // @if(a >= b, @fn(_) { Nat }, a + b, a - b)

        let mut children = Vec::new();
        children.push(BareSyntaxNode::new_leaf(
            BareSyntaxKind::Directive(directive.clone()),
            directive.to_string(),
        ));

        self.parse_column_whitespace(&mut children);
        let arg_list = self.parse_arg_list(Reparsable::Ignore);
        children.push(arg_list);
        self.parse_column_whitespace(&mut children);

        BareSyntaxNode::new_parent(BareSyntaxKind::IfValue, children)

        /*
        // Ternary (for short single-line)
        // @if(a >= b, a + b, a - b)

        // Traditional (for multi-line)
        // @if a >= b { a + b } else { a - b }
        // @if (a >= b) { a + b } else { a - b }
        // @if(a >= b){ a + b }else{ a - b }

        // With Motive (for dependent if)
        // @if(a >= b, a + b, a - b): Nat
        // @if a >= b { a + b } else { a - b }: Nat

        let mut children = Vec::new();
        children.push(BareSyntaxNode::new_leaf(
            BareSyntaxKind::Directive(directive.clone()),
            directive.to_string(),
        ));

        self.parse_column_whitespace(&mut children);

        let has_block_args = if self.lexer.peek().is_punct_of(Punct::new("(")) {
            let arg_list = self.parse_arg_list(Reparsable::Ignore);

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
                    Self::error_for_empty("Expected `else` after first block of if")
                },
            );

            self.parse_all_whitespace(&mut children);
            children.push(self.parse_block());
        }

        self.parse_column_whitespace(&mut children);
        self.parse_type_annotation(false, &mut children);
        BareSyntaxNode::new_parent(BareSyntaxKind::IfValue, children)
        */
    }

    fn parse_record_directive(&mut self, directive: Directive) -> Arc<BareSyntaxNode> {
        // @record(12, true, Type, void)

        let mut children = Vec::new();
        children.push(BareSyntaxNode::new_leaf(
            BareSyntaxKind::Directive(directive.clone()),
            directive.to_string(),
        ));

        self.parse_column_whitespace(&mut children);
        children.push(self.parse_arg_list(Reparsable::Reparse));

        BareSyntaxNode::new_parent(BareSyntaxKind::RecordValue, children)
    }

    fn parse_arg_list(&mut self, reparsable: Reparsable) -> Arc<BareSyntaxNode> {
        let mut children = vec![];
        if self
            .parse_punct(Punct::new("("), &mut children, ErrorRecovery::Empty)
            .is_ok()
        {
            self.parse_all_whitespace(&mut children);

            let mut has_param = false;

            while !self.lexer.peek().kind.is_punct_of_or_eof(Punct::new(")")) {
                if has_param {
                    let _ = self.parse_punct(
                        Punct::new(","),
                        &mut children,
                        ErrorRecovery::EatUntilNestedClosing(TokenKind::Punct(Punct::new(")"))),
                    );
                    self.parse_all_whitespace(&mut children);
                } else {
                    has_param = true;
                }

                children.push(self.parse_term());
                self.parse_all_whitespace(&mut children);
            }

            let _ = self.parse_punct(Punct::new(")"), &mut children, ErrorRecovery::Empty);
        }
        BareSyntaxNode::new_parent(BareSyntaxKind::ArgList(reparsable), children)
    }

    fn parse_field_def_list(&mut self) -> Arc<BareSyntaxNode> {
        let mut children = vec![];

        if self
            .parse_punct(Punct::new("{"), &mut children, ErrorRecovery::Empty)
            .is_ok()
        {
            self.parse_all_whitespace(&mut children);

            while !self.lexer.peek().kind.is_punct_of_or_eof(Punct::new("}")) {
                children.push(self.parse_field_def_sublist());

                let needs_separator = self.parse_all_whitespace(&mut children).is_none();

                if self.lexer.peek().kind.is_punct_of_or_eof(Punct::new("}")) {
                    break;
                } else if self.lexer.peek().is_punct_of(Punct::new(",")) {
                    let _ = self.parse_punct(
                        Punct::new(","),
                        &mut children,
                        ErrorRecovery::EatUntilNestedClosing(TokenKind::Punct(Punct::new(")"))),
                    );
                    self.parse_all_whitespace(&mut children);
                } else if needs_separator {
                    children.push(
                        self.error_for_next_token("Expected ',' or newline after field definition"),
                    );
                }
            }

            let _ = self.parse_punct(Punct::new("}"), &mut children, ErrorRecovery::Empty);
        }
        BareSyntaxNode::new_parent(BareSyntaxKind::FieldDefList, children)
    }

    fn parse_field_def_sublist(&mut self) -> Arc<BareSyntaxNode> {
        let mut children = vec![];
        self.parse_names(&mut children);
        self.parse_type_annotation(true, &mut children);
        BareSyntaxNode::new_parent(BareSyntaxKind::FieldDef, children)
    }

    fn parse_param_list(&mut self) -> Result<Arc<BareSyntaxNode>, Arc<BareSyntaxNode>> {
        let mut children = vec![];

        if self
            .parse_punct(Punct::new("("), &mut children, ErrorRecovery::Empty)
            .is_ok()
        {
            self.parse_all_whitespace(&mut children);

            let mut has_param = false;

            while !self.lexer.peek().kind.is_punct_of_or_eof(Punct::new(")")) {
                if has_param {
                    let _ = self.parse_punct(
                        Punct::new(","),
                        &mut children,
                        ErrorRecovery::EatUntilNestedClosing(TokenKind::Punct(Punct::new(")"))),
                    );
                    self.parse_all_whitespace(&mut children);
                } else {
                    has_param = true;
                }

                children.push(self.parse_param_sublist());
                self.parse_all_whitespace(&mut children);
            }

            let _ = self.parse_punct(Punct::new(")"), &mut children, ErrorRecovery::Empty);
            Ok(BareSyntaxNode::new_parent(
                BareSyntaxKind::ParamList,
                children,
            ))
        } else {
            Err(BareSyntaxNode::new_parent(
                BareSyntaxKind::ParamList,
                children,
            ))
        }
    }

    fn parse_type_annotation(&mut self, required: bool, out: &mut Vec<Arc<BareSyntaxNode>>) {
        if self.lexer.peek().is_punct_of(Punct::new(":")) {
            let mut children = vec![];
            let _ = self.parse_punct(Punct::new(":"), &mut children, ErrorRecovery::Empty);
            children.push(self.parse_term());

            out.push(BareSyntaxNode::new_parent(
                BareSyntaxKind::TypeAnnotation,
                children,
            ));
        } else if required {
            out.push(Self::error_for_empty("Expected type annotation"));
        }
    }

    fn parse_param_sublist(&mut self) -> Arc<BareSyntaxNode> {
        let mut children = vec![];
        self.parse_param_heads(&mut children);
        self.parse_type_annotation(true, &mut children);
        BareSyntaxNode::new_parent(BareSyntaxKind::Param, children)
    }

    fn parse_param_heads(&mut self, children: &mut Vec<Arc<BareSyntaxNode>>) {
        children.push(self.parse_param_head());
        self.parse_all_whitespace(children);

        while self.lexer.peek().is_punct_of(Punct::new(",")) {
            let _ = self.parse_punct(Punct::new(","), children, ErrorRecovery::Empty);
            self.parse_all_whitespace(children);
            children.push(self.parse_param_head());
            self.parse_all_whitespace(children);
        }
    }

    fn parse_param_head(&mut self) -> Arc<BareSyntaxNode> {
        let mut children = vec![];

        if let Some(implicit_name) = self.parse_implicit_name() {
            children.push(implicit_name);
            self.parse_column_whitespace(&mut children);
            children.extend(self.parse_name().into_iter());
        } else {
            children.push(self.parse_name_required());
        }

        BareSyntaxNode::new_parent(BareSyntaxKind::ParamHead, children)
    }

    fn parse_implicit_name(&mut self) -> Option<Arc<BareSyntaxNode>> {
        let mut children = vec![];
        if self
            .parse_punct(Punct::new("$"), &mut children, ErrorRecovery::Empty)
            .is_ok()
        {
            self.parse_column_whitespace(&mut children);

            if self.lexer.peek().is_identifier() {
                let identifier = self.lexer.next().kind.unwrap_identifier();
                children.push(BareSyntaxNode::new_leaf(
                    BareSyntaxKind::Identifier(identifier.clone().into()),
                    identifier,
                ));
            } else {
                children.push(self.error_until_any(
                    &[
                        TokenKind::Punct(Punct::new(",")),
                        TokenKind::Punct(Punct::new(":")),
                        TokenKind::Punct(Punct::new(")")),
                    ],
                    "Expected name for implicit argument after `$`",
                ))
            };

            Some(BareSyntaxNode::new_parent(
                BareSyntaxKind::ImplicitName,
                children,
            ))
        } else {
            None
        }
    }

    fn parse_names(&mut self, children: &mut Vec<Arc<BareSyntaxNode>>) {
        children.push(self.parse_name_required());
        self.parse_all_whitespace(children);

        while self.lexer.peek().is_punct_of(Punct::new(",")) {
            let _ = self.parse_punct(Punct::new(","), children, ErrorRecovery::Empty);
            self.parse_all_whitespace(children);
            children.push(self.parse_name_required());
            self.parse_all_whitespace(children);
        }
    }

    fn parse_whitespace(
        &mut self,
        allow_newlines: bool,
        children: &mut Vec<Arc<BareSyntaxNode>>,
    ) -> Option<LineSpacingAtom> {
        let mut has_newline = None;

        while self
            .lexer
            .eat(|token| match token.kind {
                TokenKind::ColumnSpacing(atom) => {
                    children.push(BareSyntaxNode::new_leaf(
                        BareSyntaxKind::ColumnSpacing(atom),
                        atom.to_string(),
                    ));
                    Ok(())
                }
                TokenKind::LineSpacing(atom) if allow_newlines => {
                    has_newline = Some(atom);
                    children.push(BareSyntaxNode::new_leaf(
                        BareSyntaxKind::LineSpacing(atom),
                        atom.to_string(),
                    ));
                    Ok(())
                }
                TokenKind::SinglelineComment(comment) => {
                    children.push(BareSyntaxNode::new_leaf(
                        BareSyntaxKind::SinglelineComment(comment.clone().into()),
                        comment,
                    ));
                    Ok(())
                }
                TokenKind::MultilineComment(comment, terminated) => {
                    children.push(BareSyntaxNode::new_leaf(
                        BareSyntaxKind::MultilineComment(comment.clone().into()),
                        comment,
                    ));

                    if terminated.is_unterminated() {
                        children.push(Self::error_for_empty(
                            "Expected `*/` to close multi-line comment",
                        ));
                    }

                    Ok(())
                }
                _ => Err(token),
            })
            .is_some()
        {}

        has_newline
    }

    fn parse_column_whitespace(&mut self, children: &mut Vec<Arc<BareSyntaxNode>>) {
        self.parse_whitespace(false, children);
    }

    fn parse_all_whitespace(
        &mut self,
        children: &mut Vec<Arc<BareSyntaxNode>>,
    ) -> Option<LineSpacingAtom> {
        self.parse_whitespace(true, children)
    }
}

pub fn reparse(
    document: &Document,
    _existing: Option<Arc<SyntaxNode>>,
    _range: DocumentRange,
) -> Arc<SyntaxNode> {
    let adapter = util_infinite_iterator::Adapter::new(
        document.chars().map(|c| Character::At(c, ())),
        Character::End(()),
    );

    let lexer =
        util_infinite_iterator::Peeker::new(lexer_adept::Lexer::new(CharacterPeeker::new(adapter)));

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
    let document = Document::new(r#""#.into());

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
