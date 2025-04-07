#![feature(maybe_uninit_array_assume_init)]

mod annotation;
pub mod error;
mod input;
mod make_error;
mod parse_annotation;
mod parse_block;
mod parse_enum;
mod parse_expr;
mod parse_func;
mod parse_func_params;
mod parse_global_variable;
mod parse_helper_expr;
mod parse_impl;
mod parse_stmt;
mod parse_structure;
mod parse_top_level;
mod parse_trait;
mod parse_type;
mod parse_type_alias;
mod parse_type_params;
mod parse_util;

use self::error::ParseError;
pub use self::input::Input;
use ast::AstFile;
use infinite_iterator::InfinitePeekable;
use source_files::{Source, SourceFileKey, SourceFiles};
use std::mem::MaybeUninit;
use token::{Token, TokenKind};

pub fn parse(
    tokens: impl InfinitePeekable<Token>,
    source_files: &SourceFiles,
    key: SourceFileKey,
) -> Result<AstFile, ParseError> {
    Parser::new(Input::new(tokens, source_files, key)).parse()
}

pub struct Parser<'a, I: InfinitePeekable<Token>> {
    pub input: Input<'a, I>,
    pub treat_string_literals_as_cstring_literals: bool,
}

impl<'a, I: InfinitePeekable<Token>> Parser<'a, I> {
    pub fn new(input: Input<'a, I>) -> Self {
        Self {
            input,
            treat_string_literals_as_cstring_literals: false,
        }
    }

    pub fn new_for_pragma(input: Input<'a, I>) -> Self {
        Self {
            input,
            treat_string_literals_as_cstring_literals: true,
        }
    }

    pub fn parse(&mut self) -> Result<AstFile, ParseError> {
        let mut ast_file = AstFile::new();

        // Parse into ast file
        while !self.input.peek().is_end_of_file() {
            self.parse_top_level(&mut ast_file, vec![])?;
        }

        Ok(ast_file)
    }

    fn source_here(&mut self) -> Source {
        self.input.peek().source
    }
}

fn is_terminating_token(kind: &TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::Comma | TokenKind::CloseParen | TokenKind::CloseBracket | TokenKind::CloseCurly
    )
}

fn is_right_associative(kind: &TokenKind) -> bool {
    matches!(kind, TokenKind::DeclareAssign)
}

// Const evaluation currently isn't strong enough in Rust to write a much better version of this
fn array_last<const LITTLE_N: usize, const BIG_N: usize, T: Copy>(
    big_array: [T; BIG_N],
) -> [T; LITTLE_N] {
    assert!(LITTLE_N <= BIG_N);

    let mut little_array = [const { MaybeUninit::uninit() }; LITTLE_N];

    for i in 0..LITTLE_N {
        little_array[LITTLE_N - i - 1].write(big_array[BIG_N - i - 1]);
    }

    unsafe { MaybeUninit::array_assume_init(little_array) }
}
