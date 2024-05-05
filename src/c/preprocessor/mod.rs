mod state;
mod token;

use self::token::{Encoding, PreToken, PreTokenKind, Punctuator};
use crate::{c::preprocessor::state::State, lexical_utils::IsCharacter, look_ahead::LookAhead};
use itertools::Itertools;

#[derive(Clone, Debug)]
pub enum PreprocessorError {
    UnterminatedMultiLineComment,
    UnterminatedCharacterConstant,
    UnterminatedStringLiteral,
    BadEscapeSequence,
    BadEscapedCodepoint,
}

pub fn preprocess(content: &str) -> Result<String, PreprocessorError> {
    let lines = line_splice(content);
    let tokens = lex(&lines);

    // macro_expansion();

    Ok(tokens.iter().map(|tok| format!("{:?}", tok)).join("\n"))
}

fn line_splice(content: &str) -> Vec<String> {
    let mut lines = Vec::new();
    let mut building = String::new();
    let mut chars = LookAhead::new(content.chars());

    while let Some(c) = chars.next() {
        if c == '\n' {
            lines.push(std::mem::take(&mut building));
        } else if c == '\\' && chars.peek().is_character('\n') {
            chars.next();
        } else {
            building.push(c);
        }
    }

    lines.push(std::mem::take(&mut building));
    lines
}

fn lex(lines: &Vec<String>) -> Result<Vec<PreToken>, PreprocessorError> {
    let mut tokens = Vec::new();
    let mut continuation_state = State::Idle;

    for line in lines.iter() {
        let (mut new_tokens, next_state) = lex_line(line, continuation_state)?;
        tokens.append(&mut new_tokens);
        continuation_state = next_state;
    }

    match continuation_state {
        State::MultiLineComment => Err(PreprocessorError::UnterminatedMultiLineComment),
        _ => Ok(tokens),
    }
}

fn lex_line(
    line: &str,
    starting_state: State,
) -> Result<(Vec<PreToken>, State), PreprocessorError> {
    let mut line = LookAhead::new(line.chars());
    let mut state = starting_state;
    let mut tokens = Vec::new();

    fn push_punctuator_token(tokens: &mut Vec<PreToken>, punctuator: Punctuator) {
        tokens.push(PreToken::new(PreTokenKind::Punctuator(punctuator)));
    }

    while let Some(c) = line.next() {
        match &mut state {
            State::Idle => {
                use Punctuator::*;

                match c {
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
                    // Punctuators
                    '.' if line.eat("..") => push_punctuator_token(&mut tokens, Ellipses),
                    '-' if line.eat(">") => push_punctuator_token(&mut tokens, Arrow),
                    '+' if line.eat("+") => push_punctuator_token(&mut tokens, Increment),
                    '-' if line.eat("-") => push_punctuator_token(&mut tokens, Decrement),
                    '#' if line.eat("#") => push_punctuator_token(&mut tokens, HashConcat),
                    '<' if line.eat("<") => push_punctuator_token(&mut tokens, LeftShift),
                    '>' if line.eat(">") => push_punctuator_token(&mut tokens, RightShift),
                    '!' if line.eat("=") => push_punctuator_token(&mut tokens, NotEquals),
                    '<' if line.eat("=") => push_punctuator_token(&mut tokens, LessThanEq),
                    '>' if line.eat("=") => push_punctuator_token(&mut tokens, GreaterThanEq),
                    '=' if line.eat("=") => push_punctuator_token(&mut tokens, DoubleEquals),
                    '&' if line.eat("&") => push_punctuator_token(&mut tokens, LogicalAnd),
                    '|' if line.eat("|") => push_punctuator_token(&mut tokens, LogicalOr),
                    '*' if line.eat("=") => push_punctuator_token(&mut tokens, MultiplyAssign),
                    '/' if line.eat("=") => push_punctuator_token(&mut tokens, DivideAssign),
                    '%' if line.eat("=") => push_punctuator_token(&mut tokens, ModulusAssign),
                    '+' if line.eat("=") => push_punctuator_token(&mut tokens, AddAssign),
                    '-' if line.eat("=") => push_punctuator_token(&mut tokens, SubtractAssign),
                    '<' if line.eat("<=") => push_punctuator_token(&mut tokens, LeftShiftAssign),
                    '>' if line.eat(">=") => push_punctuator_token(&mut tokens, RightShiftAssign),
                    '&' if line.eat("=") => push_punctuator_token(&mut tokens, BitAndAssign),
                    '|' if line.eat("=") => push_punctuator_token(&mut tokens, BitOrAssign),
                    '^' if line.eat("=") => push_punctuator_token(&mut tokens, BitXorAssign),
                    '[' => push_punctuator_token(&mut tokens, OpenBracket),
                    ']' => push_punctuator_token(&mut tokens, CloseBracket),
                    '(' => push_punctuator_token(&mut tokens, OpenParen),
                    ')' => push_punctuator_token(&mut tokens, CloseParen),
                    '{' => push_punctuator_token(&mut tokens, OpenCurly),
                    '}' => push_punctuator_token(&mut tokens, CloseCurly),
                    ',' => push_punctuator_token(&mut tokens, Comma),
                    ':' => push_punctuator_token(&mut tokens, Colon),
                    ';' => push_punctuator_token(&mut tokens, Semicolon),
                    '*' => push_punctuator_token(&mut tokens, Multiply),
                    '=' => push_punctuator_token(&mut tokens, Assign),
                    '#' => push_punctuator_token(&mut tokens, Hash),
                    '.' => push_punctuator_token(&mut tokens, Dot),
                    '&' => push_punctuator_token(&mut tokens, Ampersand),
                    '+' => push_punctuator_token(&mut tokens, Add),
                    '-' => push_punctuator_token(&mut tokens, Subtract),
                    '~' => push_punctuator_token(&mut tokens, BitComplement),
                    '!' => push_punctuator_token(&mut tokens, Not),
                    '/' => push_punctuator_token(&mut tokens, Divide),
                    '%' => push_punctuator_token(&mut tokens, Modulus),
                    '<' => push_punctuator_token(&mut tokens, LessThan),
                    '>' => push_punctuator_token(&mut tokens, GreaterThan),
                    '^' => push_punctuator_token(&mut tokens, BitXor),
                    '|' => push_punctuator_token(&mut tokens, BitOr),
                    '?' => push_punctuator_token(&mut tokens, Ternary),
                    // Identifiers
                    'a'..='z' | 'A'..='Z' | '_' | '$' => {
                        state = State::Identifier(c.into());
                    }
                    // Other Unrecognized Characters
                    _ => tokens.push(PreToken::new(token::PreTokenKind::Other(c))),
                }
            }
            State::Number(existing) => {
                // Yes, preprocessor numbers are weird, but this is the definition according to the C standard.
                if is_identifier_continue(c) {
                    existing.push(c);
                } else if c == '\''
                    && line
                        .peek()
                        .map_or(false, |c| c.is_ascii_digit() || is_non_digit(*c))
                {
                    existing.push(c);
                    existing.push(line.next().expect("following character to exist"));
                } else if (c == 'e' || c == 'E' || c == 'p' || c == 'P')
                    && line.peek().map_or(false, |c| is_sign(*c))
                {
                    existing.push(c);
                    existing.push(line.next().expect("following character to exist"));
                } else if c == '.' {
                    existing.push(c);
                } else {
                    tokens.push(PreToken::new(
                        state.finalize().expect("preprocessor number result"),
                    ))
                }
            }
            State::MultiLineComment => {
                if c == '*' && line.eat("/") {
                    // Close multi-line comment
                    state = State::Idle;
                }
            }
            State::Identifier(existing) => {
                if is_identifier_continue(c) {
                    existing.push(c);
                } else {
                    tokens.push(PreToken::new(
                        state.finalize().expect("preprocessor identifier result"),
                    ));
                }
            }
            State::CharacterConstant(_encoding, existing) => match c {
                '\'' => tokens.push(PreToken::new(
                    state
                        .finalize()
                        .expect("preprocessor character constant result"),
                )),
                '\\' => existing.push(escape_sequence(&mut line)?),
                _ => existing.push(c),
            },
            State::StringLiteral(_encoding, existing) => match c {
                '"' => tokens.push(PreToken::new(
                    state
                        .finalize()
                        .expect("preprocessor string literal result"),
                )),
                '\\' => existing.push(escape_sequence(&mut line)?),
                _ => existing.push(c),
            },
        }
    }

    let next_state = match state {
        State::MultiLineComment => State::MultiLineComment,
        State::CharacterConstant(..) => {
            return Err(PreprocessorError::UnterminatedCharacterConstant)
        }
        State::StringLiteral(..) => return Err(PreprocessorError::UnterminatedStringLiteral),
        _ => State::Idle,
    };

    if let Some(token_kind) = state.finalize() {
        tokens.push(PreToken::new(token_kind));
    }

    Ok((tokens, next_state))
}

fn make_character(digits: &str, radix: u32) -> Result<char, PreprocessorError> {
    let codepoint =
        u32::from_str_radix(&digits, radix).map_err(|_| PreprocessorError::BadEscapedCodepoint)?;

    char::from_u32(codepoint).ok_or(PreprocessorError::BadEscapedCodepoint)
}

fn escape_sequence<I: Iterator<Item = char>>(
    line: &mut LookAhead<I>,
) -> Result<char, PreprocessorError> {
    match line.next() {
        Some('\'') => Ok('\''),
        Some('"') => Ok('"'),
        Some('?') => Ok('?'),
        Some('\\') => Ok('\\'),
        Some('a') => Ok(0x07 as char),
        Some('b') => Ok(0x08 as char),
        Some('f') => Ok(0x0C as char),
        Some('n') => Ok('\n'),
        Some('r') => Ok('\r'),
        Some('t') => Ok('\t'),
        Some('v') => Ok(0x0B as char),
        Some('0'..='7') => {
            // Octal
            // Either \0 \00 or \000

            let mut digits = String::with_capacity(3);

            for _ in 0..3 {
                match line.next() {
                    Some(digit) if matches!(digit, '0'..='7') => digits.push(digit),
                    _ => break,
                }
            }

            make_character(&digits, 8)
        }
        Some('x') => {
            let mut digits = String::with_capacity(8);

            loop {
                match line.next() {
                    Some(digit) if digit.is_ascii_hexdigit() => digits.push(digit),
                    _ => break,
                }
            }

            make_character(&digits, 16)
        }
        Some('u') => {
            let mut digits = String::with_capacity(4);

            for _ in 0..4 {
                match line.next() {
                    Some(digit) if digit.is_ascii_hexdigit() => digits.push(digit),
                    _ => return Err(PreprocessorError::BadEscapedCodepoint),
                }
            }

            make_character(&digits, 16)
        }
        Some('U') => {
            let mut digits = String::with_capacity(8);

            for _ in 0..8 {
                match line.next() {
                    Some(digit) if digit.is_ascii_hexdigit() => digits.push(digit),
                    _ => return Err(PreprocessorError::BadEscapedCodepoint),
                }
            }

            make_character(&digits, 16)
        }
        _ => Err(PreprocessorError::BadEscapeSequence),
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

fn is_sign(c: char) -> bool {
    c == '+' || c == '-'
}
