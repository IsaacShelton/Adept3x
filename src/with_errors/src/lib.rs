mod errors;
mod top_errors;

pub use crate::{
    errors::{Error, ErrorAt},
    top_errors::TopErrors,
};
use serde::{Deserialize, Serialize};
pub use top_errors::TopErrorsNode;

#[derive(Clone, Debug, Default, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct WithErrors<T> {
    pub value: T,
    pub errors: TopErrors,
}

impl<T> WithErrors<T> {
    pub fn new(value: T, errors: TopErrors) -> Self {
        Self { value, errors }
    }

    pub fn new_one(value: T, error: Error) -> Self {
        Self {
            value,
            errors: TopErrors::new_one(ErrorAt::new(error)),
        }
    }

    pub fn new_one_at(value: T, error: ErrorAt) -> Self {
        Self {
            value,
            errors: TopErrors::new_one(error),
        }
    }

    pub fn no_errors(value: T) -> Self {
        Self {
            value,
            errors: TopErrors::default(),
        }
    }

    pub fn map<S>(self, f: impl FnOnce(T) -> S) -> WithErrors<S> {
        WithErrors {
            value: f(self.value),
            errors: self.errors,
        }
    }
}
