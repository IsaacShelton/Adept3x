use crate::resolved::{HumanName, TraitRef};
use derivative::Derivative;
use derive_more::IsVariant;
use std::fmt::Display;

#[derive(Clone, Debug, IsVariant, Derivative)]
#[derivative(PartialEq, Eq, Hash)]
pub enum Constraint {
    PrimitiveAdd,
    Trait(
        #[derivative(PartialEq = "ignore")]
        #[derivative(Hash = "ignore")]
        HumanName,
        TraitRef,
    ),
}

impl Display for Constraint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Constraint::PrimitiveAdd => write!(f, "PrimitiveAdd"),
            Constraint::Trait(name, _) => write!(f, "{}", name),
        }
    }
}
