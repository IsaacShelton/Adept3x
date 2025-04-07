use super::{Environment, depleted::Depleted};
use crate::PreprocessorError;
use pp_token::PreToken;

pub fn expand_embed(
    _options: &[PreToken],
    _environment: &mut Environment,
    _depleted: &mut Depleted,
) -> Result<Vec<PreToken>, PreprocessorError> {
    unimplemented!("#embed is not implemented yet")
}
