mod state;
mod token;

use self::token::PreToken;
use crate::{c::preprocessor::state::State, lexical_utils::IsCharacter, look_ahead::LookAhead};
use itertools::Itertools;

#[derive(Clone, Debug)]
pub enum PreprocessorError {
    UnterminatedMultiLineComment,
    UnterminatedCharacterConstant,
    UnterminatedStringLiterals,
    BadEscapeSequence,
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

    while let Some(c) = line.next() {
        match &mut state {
            State::Idle => {
                if is_digit(c) || (c == '.' && line.peek().map_or(false, |c| is_digit(*c))) {
                    state = State::Number(c.into());
                } else if c == '/' && line.peek().map_or(false, |c| *c == '/') {
                    // Line comment, ignore rest of line
                    break;
                } else if c == '/' && line.peek().map_or(false, |c| *c == '*') {
                    // Multi-line comment
                    state = State::MultiLineComment;

                    // Ignore '*' character in '/*'
                    line.next();
                } else if c == '\'' {
                    state = State::CharacterConstant(String::new());
                } else if c == '"' {
                    state = State::StringLiteral(String::new());
                } else if is_identifier_start(c) {
                    state = State::Identifier(c.into());
                } else {
                    tokens.push(PreToken::new(token::PreTokenKind::Other(c)))
                }
            }
            State::Number(existing) => {
                // Yes, preprocessor numbers are weird, but this is the definition according to the C standard.
                if is_identifier_continue(c) {
                    existing.push(c);
                } else if c == '\''
                    && line
                        .peek()
                        .map_or(false, |c| is_digit(*c) || is_non_digit(*c))
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
                if c == '*' && line.peek().map_or(false, |c| *c == '/') {
                    // Close multi-line comment
                    state = State::Idle;

                    // Ignore '/' character in '*/'
                    line.next();
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
            State::CharacterConstant(existing) => {
                if c == '\'' {
                    tokens.push(PreToken::new(
                        state
                            .finalize()
                            .expect("preprocessor character constant result"),
                    ));
                } else if c == '\\' {
                    // escape sequence
                    existing.push(escape_sequence(&mut line)?);
                } else {
                    existing.push(c);
                }
            }
            State::StringLiteral(existing) => {
                todo!()
            }
        }
    }

    let next_state = match state {
        State::MultiLineComment => State::MultiLineComment,
        State::CharacterConstant(_) => {
            return Err(PreprocessorError::UnterminatedCharacterConstant)
        }
        State::StringLiteral(_) => return Err(PreprocessorError::UnterminatedStringLiterals),
        _ => State::Idle,
    };

    if let Some(token_kind) = state.finalize() {
        tokens.push(PreToken::new(token_kind));
    }

    Ok((tokens, next_state))
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

            unimplemented!("octal escape sequences are not supported");
        }
        Some('x') => {
            // \x(hex_digit*)
            unimplemented!("hex escape sequences are not supported yet");
        }
        Some('u') => {
            // hex_quad();
            unimplemented!("universal escape sequences are not supported");
        }
        Some('U') => {
            // hex_quad();
            // hex_quad();
            unimplemented!("universal escape sequences are not supported");
        }
        _ => Err(PreprocessorError::BadEscapeSequence),
    }
}

fn is_identifier_start(c: char) -> bool {
    // NOTE: We don't handle XID_Continue character and
    // universal character names of class
    // XID_Continue
    return is_non_digit(c);
}

fn is_identifier_continue(c: char) -> bool {
    // NOTE: We don't handle XID_Continue character and
    // universal character names of class
    // XID_Continue
    return is_digit(c) || is_non_digit(c);
}

fn is_non_digit(c: char) -> bool {
    // NOTE: We support the extension of using '$' in identifier/non-digit character
    c.is_ascii_alphabetic() || c == '_' || c == '$'
}

fn is_digit(c: char) -> bool {
    c.is_ascii_digit()
}

fn is_sign(c: char) -> bool {
    c == '+' || c == '-'
}
