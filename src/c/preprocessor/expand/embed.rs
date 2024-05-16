use super::{depleted::Depleted, Environment};
use crate::c::preprocessor::{pre_token::PreToken, PreprocessorError};

pub fn expand_embed(
    _options: &[PreToken],
    _environment: &mut Environment,
    _depleted: &mut Depleted,
) -> Result<Vec<PreToken>, PreprocessorError> {
    unimplemented!("#embed is not implemented yet")
}
