use super::state::State;
use crate::{
    c::{
        encoding::Encoding,
        preprocessor::{
            error::PreprocessorErrorKind, pre_token::Punctuator, PreToken, PreTokenKind,
            PreprocessorError,
        },
    },
    source_files::Source,
    text::{is_c_non_digit, Character, Text},
};

pub fn lex_line(
    mut line: impl Text,
    starting_state: State,
) -> Result<(Vec<PreToken>, State), PreprocessorError> {
    let mut tokens = Vec::with_capacity(16);
    let mut state = starting_state;

    fn push_punctuator_token(tokens: &mut Vec<PreToken>, punctuator: Punctuator, source: Source) {
        tokens.push(PreTokenKind::Punctuator(punctuator).at(source));
    }

    fn prefer_header_name(tokens: &[PreToken]) -> bool {
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

    while let Character::At(peek_c, _) = line.peek() {
        match &mut state {
            State::Idle => {
                use Punctuator::*;

                let (c, source) = line.next().unwrap();

                match c {
                    // Whitespace
                    ' ' | '\t' | /*\v*/ '\u{0B}' | /*\f*/ '\u{0C}' => (),
                    // Numbers
                    '0'..='9' => {
                        state = State::Number(c.into(), source);
                    }
                    '.' if line.peek().is_digit() => {
                        state = State::Number(c.into(), source);
                    }
                    // Comments
                    '/' if line.eat("/") => break,
                    '/' if line.eat("*") => state = State::MultiLineComment(source),
                    // Character Literals
                    '\'' => state = State::character(Encoding::Default, source),
                    'u' if line.eat("8'") => state = State::character(Encoding::Utf8, source),
                    'u' if line.eat("'") => state = State::character(Encoding::Utf16, source),
                    'U' if line.eat("'") => state = State::character(Encoding::Utf32, source),
                    'L' if line.eat("'") => state = State::character(Encoding::Wide, source),
                    // Strings Literals
                    '"' => state = State::string(Encoding::Default, source),
                    'u' if line.eat("8\"") => state = State::string(Encoding::Utf8, source),
                    'u' if line.eat("\"") => state = State::string(Encoding::Utf16, source),
                    'U' if line.eat("\"") => state = State::string(Encoding::Utf32, source),
                    'L' if line.eat("\"") => state = State::string(Encoding::Wide, source),
                    // Header Name
                    '<' if prefer_header_name(&tokens) => state = State::HeaderName("".into(), source),
                    // Punctuators
                    '.' if line.eat("..") => push_punctuator_token(&mut tokens, Ellipses, source),
                    '-' if line.eat(">") => push_punctuator_token(&mut tokens, Arrow, source),
                    '+' if line.eat("+") => push_punctuator_token(&mut tokens, Increment, source),
                    '-' if line.eat("-") => push_punctuator_token(&mut tokens, Decrement, source),
                    '#' if line.eat("#") => push_punctuator_token(&mut tokens, HashConcat, source),
                    '<' if line.eat("<") => push_punctuator_token(&mut tokens, LeftShift, source),
                    '>' if line.eat(">") => push_punctuator_token(&mut tokens, RightShift, source),
                    '!' if line.eat("=") => push_punctuator_token(&mut tokens, NotEquals, source),
                    '<' if line.eat("=") => push_punctuator_token(&mut tokens, LessThanEq, source),
                    '>' if line.eat("=") => push_punctuator_token(&mut tokens, GreaterThanEq, source),
                    '=' if line.eat("=") => push_punctuator_token(&mut tokens, DoubleEquals, source),
                    '&' if line.eat("&") => push_punctuator_token(&mut tokens, LogicalAnd, source),
                    '|' if line.eat("|") => push_punctuator_token(&mut tokens, LogicalOr, source),
                    '*' if line.eat("=") => push_punctuator_token(&mut tokens, MultiplyAssign, source),
                    '/' if line.eat( "=") => push_punctuator_token(&mut tokens, DivideAssign, source),
                    '%' if line.eat("=") => push_punctuator_token(&mut tokens, ModulusAssign, source),
                    '+' if line.eat("=") => push_punctuator_token(&mut tokens, AddAssign, source),
                    '-' if line.eat("=") => push_punctuator_token(&mut tokens, SubtractAssign, source),
                    '<' if line.eat("<=") => push_punctuator_token(&mut tokens, LeftShiftAssign, source),
                    '>' if line.eat(">=") => push_punctuator_token(&mut tokens, RightShiftAssign, source),
                    '&' if line.eat( "=") => push_punctuator_token(&mut tokens, BitAndAssign, source),
                    '|' if line.eat("=") => push_punctuator_token(&mut tokens, BitOrAssign, source),
                    '^' if line.eat("=") => push_punctuator_token(&mut tokens, BitXorAssign, source),
                    '[' => push_punctuator_token(&mut tokens, OpenBracket, source),
                    ']' => push_punctuator_token(&mut tokens, CloseBracket, source),
                    '(' => push_punctuator_token(&mut tokens, OpenParen { preceeded_by_whitespace }, source),
                    ')' => push_punctuator_token(&mut tokens, CloseParen, source),
                    '{' => push_punctuator_token(&mut tokens, OpenCurly, source),
                    '}' => push_punctuator_token(&mut tokens, CloseCurly, source),
                    ',' => push_punctuator_token(&mut tokens, Comma, source),
                    ':' => push_punctuator_token(&mut tokens, Colon, source),
                    ';' => push_punctuator_token(&mut tokens, Semicolon, source),
                    '*' => push_punctuator_token(&mut tokens, Multiply, source),
                    '=' => push_punctuator_token(&mut tokens, Assign, source),
                    '#' => push_punctuator_token(&mut tokens, Hash, source),
                    '.' => push_punctuator_token(&mut tokens, Dot, source),
                    '&' => push_punctuator_token(&mut tokens, Ampersand, source),
                    '+' => push_punctuator_token(&mut tokens, Add, source),
                    '-' => push_punctuator_token(&mut tokens, Subtract, source),
                    '~' => push_punctuator_token(&mut tokens, BitComplement, source),
                    '!' => push_punctuator_token(&mut tokens, Not, source),
                    '/' => push_punctuator_token(&mut tokens, Divide, source),
                    '%' => push_punctuator_token(&mut tokens, Modulus, source),
                    '<' => push_punctuator_token(&mut tokens, LessThan, source),
                    '>' => push_punctuator_token(&mut tokens, GreaterThan, source),
                    '^' => push_punctuator_token(&mut tokens, BitXor, source),
                    '|' => push_punctuator_token(&mut tokens, BitOr, source),
                    '?' => push_punctuator_token(&mut tokens, Ternary, source),
                    // Identifiers
                    'a'..='z' | 'A'..='Z' | '_' | '$' => {
                        state = State::Identifier(c.into(), source);
                    }
                    // Other Unrecognized Characters
                    _ => tokens.push(PreTokenKind::Other(c).at(source)),
                }

                preceeded_by_whitespace = match c {
                    ' ' | '\t' | /*\v*/ '\u{0B}' | /*\f*/ '\u{0C}' => true,
                    _ => false,
                };
            }
            State::Number(existing, _source) => {
                // Yes, preprocessor numbers are weird, but this is the definition according to the C standard.
                let next = line.peek_nth(1);

                match peek_c {
                    '\'' if next.is_digit() || next.is_c_non_digit() => {
                        existing.push(line.next().expect("digit separator").0);
                        existing.push(line.next().expect("following digit character to exist").0);
                    }
                    'e' | 'E' | 'p' | 'P' if next.is_sign() => {
                        existing.push(line.next().expect("exponent marker").0);
                        existing.push(line.next().expect("following sign character to exist").0);
                    }
                    'a'..='z' | 'A'..='Z' | '0'..='9' | '_' | '$' | '.' => {
                        existing.push(line.next().unwrap().0)
                    }
                    _ => tokens.push(state.finalize().expect("number")),
                }
            }
            State::MultiLineComment(_) => {
                if line.eat("*/") {
                    state = State::Idle;
                } else {
                    line.next();
                }
            }
            State::Identifier(existing, _source) => {
                if is_identifier_continue(peek_c) {
                    existing.push(line.next().expect("identifier character").0);
                } else {
                    tokens.push(state.finalize().expect("identifier"));
                }
            }
            State::CharacterConstant(_encoding, existing, _source) => match line.next().unwrap() {
                ('\'', _) => tokens.push(state.finalize().expect("character constant")),
                ('\\', source) => existing.push(escape_sequence(&mut line, source)?),
                (character, _) => existing.push(character),
            },
            State::StringLiteral(_encoding, existing, _source) => match line.next().unwrap() {
                ('"', _) => tokens.push(state.finalize().expect("string literal")),
                ('\\', source) => existing.push(escape_sequence(&mut line, source)?),
                (character, _) => existing.push(character),
            },
            State::HeaderName(existing, _source) => match line.next().unwrap().0 {
                '>' => tokens.push(state.finalize().expect("header name")),
                character => existing.push(character),
            },
        }
    }

    let next_state = match state {
        State::MultiLineComment(source) => Ok(State::MultiLineComment(source)),
        State::CharacterConstant(_, _, source) => {
            Err(PreprocessorErrorKind::UnterminatedCharacterConstant.at(source))
        }
        State::StringLiteral(_, _, source) => {
            Err(PreprocessorErrorKind::UnterminatedStringLiteral.at(source))
        }
        State::HeaderName(_, source) => {
            Err(PreprocessorErrorKind::UnterminatedHeaderName.at(source))
        }
        _ => Ok(State::Idle),
    }?;

    if let Some(token) = state.finalize() {
        tokens.push(token);
    }

    Ok((tokens, next_state))
}

fn make_character(digits: &str, radix: u32, source: Source) -> Result<char, PreprocessorError> {
    u32::from_str_radix(digits, radix)
        .ok()
        .and_then(char::from_u32)
        .ok_or_else(|| PreprocessorErrorKind::BadEscapedCodepoint.at(source))
}

fn escape_sequence(line: &mut impl Text, char_source: Source) -> Result<char, PreprocessorError> {
    match line.next() {
        Character::At('\'', _) => Ok('\''),
        Character::At('"', _) => Ok('"'),
        Character::At('?', _) => Ok('?'),
        Character::At('\\', _) => Ok('\\'),
        Character::At('a', _) => Ok('\u{07}'),
        Character::At('b', _) => Ok('\u{08}'),
        Character::At('f', _) => Ok('\u{0C}'),
        Character::At('n', _) => Ok('\n'),
        Character::At('r', _) => Ok('\r'),
        Character::At('t', _) => Ok('\t'),
        Character::At('v', _) => Ok('\u{0B}'),
        Character::At(start_digit @ '0'..='7', _) => {
            // Octal - Either \0 \00 or \000

            let mut digits = String::with_capacity(3);
            digits.push(start_digit);

            for _ in 0..2 {
                match line.peek() {
                    Character::At('0'..='7', _) => digits.push(line.next().unwrap().0),
                    _ => break,
                }
            }

            make_character(&digits, 8, char_source)
        }
        Character::At('x', _) => {
            let mut digits = String::with_capacity(8);

            loop {
                match line.peek() {
                    Character::At(digit, _) if digit.is_ascii_hexdigit() => {
                        digits.push(line.next().unwrap().0)
                    }
                    _ => break,
                }
            }

            make_character(&digits, 16, char_source)
        }
        Character::At('u', _) => {
            let mut digits = String::with_capacity(4);

            for _ in 0..4 {
                match line.next() {
                    Character::At(digit, _) if digit.is_ascii_hexdigit() => digits.push(digit),
                    bad => return Err(PreprocessorErrorKind::BadEscapedCodepoint.at(bad.source())),
                }
            }

            make_character(&digits, 16, char_source)
        }
        Character::At('U', _) => {
            let mut digits = String::with_capacity(8);

            for _ in 0..8 {
                match line.next() {
                    Character::At(digit, _) if digit.is_ascii_hexdigit() => digits.push(digit),
                    bad => return Err(PreprocessorErrorKind::BadEscapedCodepoint.at(bad.source())),
                }
            }

            make_character(&digits, 16, char_source)
        }
        bad => Err(PreprocessorErrorKind::BadEscapeSequence.at(bad.source())),
    }
}

fn is_identifier_continue(c: char) -> bool {
    // NOTE: We don't handle XID_Continue character and
    // universal character names of class
    // XID_Continue
    c.is_ascii_digit() || is_c_non_digit(c)
}
