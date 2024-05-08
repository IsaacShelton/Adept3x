use super::{depleted::Depleted, Environment};
use crate::c::preprocessor::{
    ast::DefineKind,
    pre_token::{PreToken, PreTokenKind},
    PreprocessorError,
};

pub fn expand_region(
    tokens: &[PreToken],
    environment: &Environment,
    depleted: &mut Depleted,
) -> Result<Vec<PreToken>, PreprocessorError> {
    let mut expanded = Vec::with_capacity(tokens.len() + 16);

    for token in tokens.iter() {
        expanded.append(&mut expand_token(token, environment, depleted)?);
    }

    Ok(expanded)
}

fn expand_token(
    token: &PreToken,
    environment: &Environment,
    depleted: &mut Depleted,
) -> Result<Vec<PreToken>, PreprocessorError> {
    match &token.kind {
        PreTokenKind::Identifier(name) => {
            if let Some(define) = environment.find_define(name, None) {
                let hash = Depleted::hash_define(define);

                if !depleted.contains(hash) {
                    depleted.push(hash);
                    let replacement = match &define.kind {
                        DefineKind::Normal(replacement) => replacement,
                        DefineKind::Macro(_) => unimplemented!("expanding macro define"),
                    };
                    let expanded = expand_region(&replacement, environment, depleted)?;
                    depleted.pop(hash);
                    return Ok(expanded);
                }
            }

            Ok(vec![token.clone()])
        }
        PreTokenKind::HeaderName(_)
        | PreTokenKind::Number(_)
        | PreTokenKind::CharacterConstant(_, _)
        | PreTokenKind::StringLiteral(_, _)
        | PreTokenKind::Punctuator(_)
        | PreTokenKind::UniversalCharacterName(_)
        | PreTokenKind::Other(_) => Ok(vec![token.clone()]),
    }
}
