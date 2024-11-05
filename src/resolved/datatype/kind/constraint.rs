use std::fmt::Display;

#[derive(Clone, Debug, PartialEq)]
pub enum Constraint {
    Add,
}

impl Display for Constraint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Add")
    }
}
