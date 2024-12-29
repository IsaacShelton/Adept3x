use crate::resolved::{HumanName, TraitRef, Type};
use derivative::Derivative;
use derive_more::IsVariant;
use itertools::Itertools;
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
        Vec<Type>,
    ),
}

impl Display for Constraint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Constraint::PrimitiveAdd => write!(f, "PrimitiveAdd"),
            Constraint::Trait(name, _, arguments) => {
                write!(f, "{}", name)?;

                if !arguments.is_empty() {
                    write!(f, "<")?;

                    let inner = arguments.iter().map(|x| x.to_string()).join(", ");
                    write!(f, "{}", inner)?;

                    write!(f, ">")?;
                }

                Ok(())
            }
        }
    }
}
