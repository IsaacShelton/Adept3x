use crate::{
    name::{Name, ResolvedName},
    resolve::error::{ResolveError, ResolveErrorKind},
    source_files::Source,
};

#[derive(Clone, Debug)]
pub enum FindTypeError {
    NotDefined,
    Ambiguous,
    RecursiveAlias(ResolvedName),
    ResolveError(ResolveError),
    ConstraintsNotSatisfied,
    TypeArgsLengthMismatch,
}

impl FindTypeError {
    pub fn into_resolve_error(self: FindTypeError, name: &Name, source: Source) -> ResolveError {
        let name = name.to_string();

        match self {
            FindTypeError::NotDefined => ResolveErrorKind::UndeclaredType {
                name: name.to_string(),
            }
            .at(source),
            FindTypeError::Ambiguous => ResolveErrorKind::AmbiguousType {
                name: name.to_string(),
            }
            .at(source),
            FindTypeError::RecursiveAlias(_) => ResolveErrorKind::RecursiveTypeAlias {
                name: name.to_string(),
            }
            .at(source),
            FindTypeError::ConstraintsNotSatisfied => {
                ResolveErrorKind::ConstraintsNotSatisfiedForType {
                    name: name.to_string(),
                }
                .at(source)
            }
            FindTypeError::TypeArgsLengthMismatch => ResolveErrorKind::Other {
                message: "Incorrect number of type arguments for type".into(),
            }
            .at(source),
            FindTypeError::ResolveError(err) => err,
        }
    }
}
