use std::fmt::Display;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Constraint {
    PrimitiveAdd,
}

impl Display for Constraint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Constraint::PrimitiveAdd => write!(f, "PrimitiveAdd"),
        }
    }
}
