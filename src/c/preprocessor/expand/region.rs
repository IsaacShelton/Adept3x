use super::{depleted::Depleted, Environment};
use crate::{
    c::preprocessor::{
        pre_token::{PreToken, PreTokenKind, Punctuator},
        ParseError, PreprocessorError,
    },
    look_ahead::LookAhead,
};

pub fn expand_region(
    pre_tokens: &[PreToken],
    environment: &Environment,
    depleted: &mut Depleted,
) -> Result<Vec<PreToken>, PreprocessorError> {
    let mut expanded = Vec::with_capacity(pre_tokens.len() + 16);
    let mut tokens = LookAhead::new(pre_tokens.iter());

    while let Some(token) = tokens.next() {
        expand_token(token, &mut tokens, environment, depleted, &mut expanded)?;
    }

    Ok(expanded)
}

fn expand_token<'a>(
    token: &PreToken,
    tokens: &mut LookAhead<impl Iterator<Item = &'a PreToken>>,
    environment: &Environment,
    depleted: &mut Depleted,
    expanded: &mut Vec<PreToken>,
) -> Result<(), PreprocessorError> {
    match &token.kind {
        PreTokenKind::Identifier(name) => {
            if let Some(PreToken {
                kind: PreTokenKind::Punctuator(Punctuator::OpenParen { .. }),
            }) = tokens.peek()
            {
                return expand_macro(name, tokens, environment, depleted, expanded);
            }

            if let Some(define) = environment.find_define(name, None) {
                let hash = Depleted::hash_define(define);

                if !depleted.contains(hash) {
                    let replacement = define.kind.clone().unwrap_normal();

                    depleted.push(hash);
                    expanded.append(&mut expand_region(&replacement, environment, depleted)?);
                    depleted.pop(hash);

                    return Ok(());
                }
            }

            expanded.push(token.clone());
            Ok(())
        }
        PreTokenKind::HeaderName(_)
        | PreTokenKind::Number(_)
        | PreTokenKind::CharacterConstant(_, _)
        | PreTokenKind::StringLiteral(_, _)
        | PreTokenKind::Punctuator(_)
        | PreTokenKind::UniversalCharacterName(_)
        | PreTokenKind::Other(_) => {
            expanded.push(token.clone());
            Ok(())
        }
    }
}

fn expand_macro<'a>(
    _name: &str,
    tokens: &mut LookAhead<impl Iterator<Item = &'a PreToken>>,
    _environment: &Environment,
    _depleted: &mut Depleted,
    _expanded: &mut Vec<PreToken>,
) -> Result<(), PreprocessorError> {
    // Eat '('
    tokens.next().unwrap();

    Err(PreprocessorError::ParseError(
        ParseError::ExpectedCloseParen,
    ))
}
