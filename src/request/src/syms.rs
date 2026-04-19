use crate::{Error, TopErrors};
use serde::{Deserialize, Serialize};

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
            errors: TopErrors::new_one(error),
        }
    }

    pub fn no_errors(value: T) -> Self {
        Self {
            value,
            errors: TopErrors::default(),
        }
    }
}
