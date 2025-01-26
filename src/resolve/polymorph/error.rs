use crate::source_files::Source;
use std::fmt::Display;

#[derive(Clone, Debug)]
pub struct PolymorphError {
    pub kind: PolymorphErrorKind,
    pub source: Source,
}

#[derive(Clone, Debug)]
pub enum PolymorphErrorKind {
    UndefinedPolymorph(String),
    PolymorphIsNotAType(String),
    PolymorphIsNotAnImpl(String),
}

impl PolymorphErrorKind {
    pub fn at(self, source: Source) -> PolymorphError {
        PolymorphError { kind: self, source }
    }
}

impl Display for PolymorphErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PolymorphErrorKind::UndefinedPolymorph(name) => {
                write!(f, "Undefined polymorph '${}'", name)
            }
            PolymorphErrorKind::PolymorphIsNotAType(name) => {
                write!(f, "Polymorph '${}' is not a type", name)
            }
            PolymorphErrorKind::PolymorphIsNotAnImpl(name) => {
                write!(f, "Polymorph '${}' is not a trait implementation", name)
            }
        }
    }
}
