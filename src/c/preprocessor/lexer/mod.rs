mod state;

use std::num::NonZeroU32;

use self::state::State;
use super::{line_splice::Line, pre_token::PreToken, PreprocessorError, PreprocessorErrorKind};
use crate::{
    c::{
        encoding::Encoding,
        preprocessor::pre_token::{PreTokenKind, Punctuator},
    },
    look_ahead::LookAhead,
};

pub fn lex(lines: impl Iterator<Item = Line>) -> Result<Vec<Vec<PreToken>>, PreprocessorError> {
    let mut tokens = Vec::new();
    let mut continuation_state = State::Idle;

    for line in lines {
        let (new_tokens, next_state) = lex_line(&line, continuation_state)?;
        tokens.push(new_tokens);
        continuation_state = next_state;
    }

    match continuation_state {
        State::MultiLineComment => Err(PreprocessorErrorKind::UnterminatedMultiLineComment.at(None)),
        _ => Ok(tokens),
    }
}

fn lex_line(
    line: &Line,
    starting_state: State,
) -> Result<(Vec<PreToken>, State), PreprocessorError> {
    let line_number = line.line_number;
    let mut line = LookAhead::new(line.content.chars());
    let mut state = starting_state;
    let mut tokens = Vec::with_capacity(16);

    fn push_punctuator_token(
        tokens: &mut Vec<PreToken>,
        punctuator: Punctuator,
        line_number: NonZeroU32,
    ) {
        tokens.push(PreToken::new(
            PreTokenKind::Punctuator(punctuator),
            Some(line_number),
        ));
    }

    fn prefer_header_name(tokens: &Vec<PreToken>) -> bool {
        if tokens.len() < 2 {
            return false;
        }

        let a = &tokens[tokens.len() - 2];
        let b = &tokens[tokens.len() - 1];

        // `#include` and `#embed`
        if a.is_hash() && (b.is_identifier("include") || b.is_identifier("embed")) {
            return true;
        }

        // `__has_include(` and `__has_embed(`
        if (a.is_identifier("__has_include") || a.is_identifier("__has_embed"))
            && b.is_open_paren_disregard_whitespace()
        {
            return true;
        }

        false
    }

    let mut preceeded_by_whitespace = true;

    while let Some(peek_c) = line.peek() {
        match &mut state {
            State::Idle => {
                use Punctuator::*;

                let c = line.next().unwrap();

                match c {
                    // Whitespace
                    ' ' | '\t' | /*\v*/ '\u{0B}' | /*\f*/ '\u{0C}' => (),
                    // Numbers
                    '0'..='9' => {
                        state = State::Number(c.into());
                    }
                    '.' if line.peek().map_or(false, char::is_ascii_digit) => {
                        state = State::Number(c.into());
                    }
                    // Comments
                    '/' if line.eat("/") => break,
                    '/' if line.eat("*") => state = State::MultiLineComment,
                    // Character Literals
                    '\'' => state = State::character(Encoding::Default),
                    'u' if line.eat("8'") => state = State::character(Encoding::Utf8),
                    'u' if line.eat("'") => state = State::character(Encoding::Utf16),
                    'U' if line.eat("'") => state = State::character(Encoding::Utf32),
                    'L' if line.eat("'") => state = State::character(Encoding::Wide),
                    // Strings Literals
                    '"' => state = State::string(Encoding::Default),
                    'u' if line.eat("8\"") => state = State::string(Encoding::Utf8),
                    'u' if line.eat("\"") => state = State::string(Encoding::Utf16),
                    'U' if line.eat("\"") => state = State::string(Encoding::Utf32),
                    'L' if line.eat("\"") => state = State::string(Encoding::Wide),
                    // Header Name
                    '<' if prefer_header_name(&tokens) => state = State::HeaderName("".into()),
                    // Punctuators
                    '.' if line.eat("..") => push_punctuator_token(&mut tokens, Ellipses, line_number),
                    '-' if line.eat(">") => push_punctuator_token(&mut tokens, Arrow, line_number),
                    '+' if line.eat("+") => push_punctuator_token(&mut tokens, Increment, line_number),
                    '-' if line.eat("-") => push_punctuator_token(&mut tokens, Decrement, line_number),
                    '#' if line.eat("#") => push_punctuator_token(&mut tokens, HashConcat, line_number),
                    '<' if line.eat("<") => push_punctuator_token(&mut tokens, LeftShift, line_number),
                    '>' if line.eat(">") => push_punctuator_token(&mut tokens, RightShift, line_number),
                    '!' if line.eat("=") => push_punctuator_token(&mut tokens, NotEquals, line_number),
                    '<' if line.eat("=") => push_punctuator_token(&mut tokens, LessThanEq, line_number),
                    '>' if line.eat("=") => push_punctuator_token(&mut tokens, GreaterThanEq, line_number),
                    '=' if line.eat("=") => push_punctuator_token(&mut tokens, DoubleEquals, line_number),
                    '&' if line.eat("&") => push_punctuator_token(&mut tokens, LogicalAnd, line_number),
                    '|' if line.eat("|") => push_punctuator_token(&mut tokens, LogicalOr, line_number),
                    '*' if line.eat("=") => push_punctuator_token(&mut tokens, MultiplyAssign, line_number),
                    '/' if line.eat("=") => push_punctuator_token(&mut tokens, DivideAssign, line_number),
                    '%' if line.eat("=") => push_punctuator_token(&mut tokens, ModulusAssign, line_number),
                    '+' if line.eat("=") => push_punctuator_token(&mut tokens, AddAssign, line_number),
                    '-' if line.eat("=") => push_punctuator_token(&mut tokens, SubtractAssign, line_number),
                    '<' if line.eat("<=") => push_punctuator_token(&mut tokens, LeftShiftAssign, line_number),
                    '>' if line.eat(">=") => push_punctuator_token(&mut tokens, RightShiftAssign, line_number),
                    '&' if line.eat("=") => push_punctuator_token(&mut tokens, BitAndAssign, line_number),
                    '|' if line.eat("=") => push_punctuator_token(&mut tokens, BitOrAssign, line_number),
                    '^' if line.eat("=") => push_punctuator_token(&mut tokens, BitXorAssign, line_number),
                    '[' => push_punctuator_token(&mut tokens, OpenBracket, line_number),
                    ']' => push_punctuator_token(&mut tokens, CloseBracket, line_number),
                    '(' => push_punctuator_token(&mut tokens, OpenParen { preceeded_by_whitespace }, line_number),
                    ')' => push_punctuator_token(&mut tokens, CloseParen, line_number),
                    '{' => push_punctuator_token(&mut tokens, OpenCurly, line_number),
                    '}' => push_punctuator_token(&mut tokens, CloseCurly, line_number),
                    ',' => push_punctuator_token(&mut tokens, Comma, line_number),
                    ':' => push_punctuator_token(&mut tokens, Colon, line_number),
                    ';' => push_punctuator_token(&mut tokens, Semicolon, line_number),
                    '*' => push_punctuator_token(&mut tokens, Multiply, line_number),
                    '=' => push_punctuator_token(&mut tokens, Assign, line_number),
                    '#' => push_punctuator_token(&mut tokens, Hash, line_number),
                    '.' => push_punctuator_token(&mut tokens, Dot, line_number),
                    '&' => push_punctuator_token(&mut tokens, Ampersand, line_number),
                    '+' => push_punctuator_token(&mut tokens, Add, line_number),
                    '-' => push_punctuator_token(&mut tokens, Subtract, line_number),
                    '~' => push_punctuator_token(&mut tokens, BitComplement, line_number),
                    '!' => push_punctuator_token(&mut tokens, Not, line_number),
                    '/' => push_punctuator_token(&mut tokens, Divide, line_number),
                    '%' => push_punctuator_token(&mut tokens, Modulus, line_number),
                    '<' => push_punctuator_token(&mut tokens, LessThan, line_number),
                    '>' => push_punctuator_token(&mut tokens, GreaterThan, line_number),
                    '^' => push_punctuator_token(&mut tokens, BitXor, line_number),
                    '|' => push_punctuator_token(&mut tokens, BitOr, line_number),
                    '?' => push_punctuator_token(&mut tokens, Ternary, line_number),
                    // Identifiers
                    'a'..='z' | 'A'..='Z' | '_' | '$' => {
                        state = State::Identifier(c.into());
                    }
                    // Other Unrecognized Characters
                    _ => tokens.push(PreToken::new(PreTokenKind::Other(c), Some(line_number))),
                }

                preceeded_by_whitespace = match c {
                    ' ' | '\t' | /*\v*/ '\u{0B}' | /*\f*/ '\u{0C}' => true,
                    _ => false,
                };
            }
            State::Number(existing) => {
                // Yes, preprocessor numbers are weird, but this is the definition according to the C standard.
                let peek_c = *peek_c;
                let next = line.peek_nth(1);

                match peek_c {
                    '\'' if next.map_or(false, |c| c.is_ascii_digit() || is_non_digit(*c)) => {
                        existing.push(line.next().expect("digit separator"));
                        existing.push(line.next().expect("following digit character to exist"));
                    }
                    'e' | 'E' | 'p' | 'P' if next.map_or(false, |c| matches!(c, '+' | '-')) => {
                        existing.push(line.next().expect("exponent marker"));
                        existing.push(line.next().expect("following sign character to exist"));
                    }
                    'a'..='z' | 'A'..='Z' | '0'..='9' | '_' | '$' | '.' => {
                        existing.push(line.next().unwrap())
                    }
                    _ => tokens.push(PreToken::new(
                        state.finalize().expect("number"),
                        Some(line_number),
                    )),
                }
            }
            State::MultiLineComment => {
                if line.eat("*/") {
                    // Close multi-line comment
                    state = State::Idle;
                } else {
                    line.next().expect("character ignored by comment");
                }
            }
            State::Identifier(existing) => {
                if is_identifier_continue(*peek_c) {
                    existing.push(line.next().expect("identifier character"));
                } else {
                    tokens.push(PreToken::new(
                        state.finalize().expect("identifier"),
                        Some(line_number),
                    ));
                }
            }
            State::CharacterConstant(_encoding, existing) => match line.next().unwrap() {
                '\'' => tokens.push(PreToken::new(
                    state.finalize().expect("character constant"),
                    Some(line_number),
                )),
                '\\' => existing.push(escape_sequence(&mut line, line_number)?),
                character => existing.push(character),
            },
            State::StringLiteral(_encoding, existing) => match line.next().unwrap() {
                '"' => tokens.push(PreToken::new(
                    state.finalize().expect("string literal"),
                    Some(line_number),
                )),
                '\\' => existing.push(escape_sequence(&mut line, line_number)?),
                character => existing.push(character),
            },
            State::HeaderName(existing) => match line.next().unwrap() {
                '>' => tokens.push(PreToken::new(
                    state.finalize().expect("header name"),
                    Some(line_number),
                )),
                character => existing.push(character),
            },
        }
    }

    let next_state = match state {
        State::MultiLineComment => Ok(State::MultiLineComment),
        State::CharacterConstant(..) => Err(PreprocessorErrorKind::UnterminatedCharacterConstant.at(Some(line_number))),
        State::StringLiteral(..) => Err(PreprocessorErrorKind::UnterminatedStringLiteral.at(Some(line_number))),
        State::HeaderName(..) => Err(PreprocessorErrorKind::UnterminatedHeaderName.at(Some(line_number))),
        _ => Ok(State::Idle),
    }?;

    if let Some(token_kind) = state.finalize() {
        tokens.push(PreToken::new(token_kind, Some(line_number)));
    }

    Ok((tokens, next_state))
}

fn make_character(digits: &str, radix: u32, line_number: NonZeroU32) -> Result<char, PreprocessorError> {
    let codepoint = u32::from_str_radix(&digits, radix)
        .map_err(|_| PreprocessorErrorKind::BadEscapedCodepoint.at(Some(line_number)))?;

    char::from_u32(codepoint).ok_or(PreprocessorErrorKind::BadEscapedCodepoint.at(Some(line_number)))
}

fn escape_sequence<I: Iterator<Item = char>>(
    line: &mut LookAhead<I>,
    line_number: NonZeroU32,
) -> Result<char, PreprocessorError> {
    match line.next() {
        Some('\'') => Ok('\''),
        Some('"') => Ok('"'),
        Some('?') => Ok('?'),
        Some('\\') => Ok('\\'),
        Some('a') => Ok('\u{07}'),
        Some('b') => Ok('\u{08}'),
        Some('f') => Ok('\u{0C}'),
        Some('n') => Ok('\n'),
        Some('r') => Ok('\r'),
        Some('t') => Ok('\t'),
        Some('v') => Ok('\u{0B}'),
        Some(start_digit @ '0'..='7') => {
            // Octal - Either \0 \00 or \000

            let mut digits = String::with_capacity(3);
            digits.push(start_digit);

            for _ in 0..2 {
                match line.peek() {
                    Some(digit) if matches!(digit, '0'..='7') => digits.push(line.next().unwrap()),
                    _ => break,
                }
            }

            make_character(&digits, 8, line_number)
        }
        Some('x') => {
            let mut digits = String::with_capacity(8);

            loop {
                match line.peek() {
                    Some(digit) if digit.is_ascii_hexdigit() => digits.push(line.next().unwrap()),
                    _ => break,
                }
            }

            make_character(&digits, 16, line_number)
        }
        Some('u') => {
            let mut digits = String::with_capacity(4);

            for _ in 0..4 {
                match line.next() {
                    Some(digit) if digit.is_ascii_hexdigit() => digits.push(digit),
                    _ => return Err(PreprocessorErrorKind::BadEscapedCodepoint.at(Some(line_number))),
                }
            }

            make_character(&digits, 16, line_number)
        }
        Some('U') => {
            let mut digits = String::with_capacity(8);

            for _ in 0..8 {
                match line.next() {
                    Some(digit) if digit.is_ascii_hexdigit() => digits.push(digit),
                    _ => return Err(PreprocessorErrorKind::BadEscapedCodepoint.at(Some(line_number))),
                }
            }

            make_character(&digits, 16, line_number)
        }
        _ => Err(PreprocessorErrorKind::BadEscapeSequence.at(Some(line_number))),
    }
}

fn is_identifier_continue(c: char) -> bool {
    // NOTE: We don't handle XID_Continue character and
    // universal character names of class
    // XID_Continue
    return c.is_ascii_digit() || is_non_digit(c);
}

fn is_non_digit(c: char) -> bool {
    // NOTE: We support the extension of using '$' in identifier/non-digit character
    c.is_ascii_alphabetic() || c == '_' || c == '$'
}
