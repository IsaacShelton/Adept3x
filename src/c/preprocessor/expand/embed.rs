use super::{depleted::Depleted, Environment, Token};
use crate::c::preprocessor::{pre_token::PreToken, PreprocessorError};

pub fn expand_embed(
    _options: &[PreToken],
    _environment: &mut Environment,
    _depleted: &mut Depleted,
) -> Result<Vec<Token>, PreprocessorError> {
    unimplemented!("#embed is not implemented yet")
}
