use crate::{
    resolve::PolyRecipe,
    resolved::{Destination, StructureRef, Type},
};

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct Member {
    pub subject: Destination,
    pub structure_ref: StructureRef,
    pub poly_recipe: PolyRecipe,
    pub index: usize,
    pub field_type: Type,
}
