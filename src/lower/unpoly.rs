use super::error::LowerError;
use crate::resolved::{self, PolyRecipe};

pub fn unpoly(poly_recipe: &PolyRecipe, ty: &resolved::Type) -> Result<resolved::Type, LowerError> {
    poly_recipe.resolve_type(ty).map_err(LowerError::from)
}
