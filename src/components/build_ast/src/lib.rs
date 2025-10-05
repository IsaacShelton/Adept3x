#![feature(maybe_uninit_array_assume_init)]

mod annotation;
pub mod error;
mod file_header;
mod input;
mod make_error;
mod parse_annotation;
mod parse_block;
mod parse_enum;
mod parse_expr;
mod parse_func;
mod parse_func_params;
mod parse_global;
mod parse_helper_expr;
mod parse_impl;
mod parse_linkset;
mod parse_namespace;
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
use ast::{ConformBehavior, RawAstFile};
use error::ParseErrorKind;
use infinite_iterator::InfinitePeekable;
use primitives::CIntegerAssumptions;
use source_files::{SourceFileKey, SourceFiles};
use std::mem::MaybeUninit;
use token::{Token, TokenKind};

pub fn parse(
    tokens: impl InfinitePeekable<Token>,
    source_files: &SourceFiles,
    key: SourceFileKey,
    conform_behavior: ConformBehavior,
) -> Result<RawAstFile, ParseError> {
    Parser::new(Input::new(tokens, source_files, key), conform_behavior).parse()
}

pub struct Parser<'a, I: InfinitePeekable<Token>> {
    pub input: Input<'a, I>,
    pub treat_string_literals_as_cstring_literals: bool,
    pub conform_behavior: ConformBehavior,
}

impl<'a, I: InfinitePeekable<Token>> Parser<'a, I> {
    pub fn new(input: Input<'a, I>, conform_behavior: ConformBehavior) -> Self {
        Self {
            input,
            treat_string_literals_as_cstring_literals: false,
            conform_behavior,
        }
    }

    pub fn new_for_pragma(input: Input<'a, I>) -> Self {
        Self {
            input,
            treat_string_literals_as_cstring_literals: true,
            conform_behavior: ConformBehavior::Adept(CIntegerAssumptions::default()),
        }
    }

    pub fn parse(&mut self) -> Result<RawAstFile, ParseError> {
        let ast_file = self.parse_namespace_items()?;

        if !self.input.peek().is_end_of_file() {
            return Err(ParseErrorKind::UnexpectedToken {
                unexpected: self.input.peek().to_string(),
            }
            .at(self.input.here()));
        }

        Ok(ast_file)
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
