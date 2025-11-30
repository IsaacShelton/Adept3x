use crate::Error;
use std::{fmt::Display, sync::Arc};
use top_n::TopN;

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct TopErrors {
    top_n: Arc<TopN<Error>>,
}

impl TopErrors {
    pub fn push(&mut self, error: Error) -> &mut Self {
        Arc::make_mut(&mut self.top_n).push(error, |a, b| a.cmp(b));
        self
    }
}

impl Default for TopErrors {
    fn default() -> Self {
        Self {
            top_n: Arc::new(TopN::new(1)),
        }
    }
}

impl Display for TopErrors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for error in self.top_n.iter() {
            writeln!(f, "{}", error)?;
        }
        Ok(())
    }
}

impl From<Error> for TopErrors {
    fn from(value: Error) -> Self {
        Self {
            top_n: Arc::new(TopN::from_iter(1, std::iter::once(value), |a, b| a.cmp(b))),
        }
    }
}

impl<T, S> Into<Result<Result<T, TopErrors>, S>> for Error {
    fn into(self) -> Result<Result<T, TopErrors>, S> {
        Ok(Err(TopErrors::from(self)))
    }
}

#[macro_export]
macro_rules! try_ok {
    ($expr:expr) => {
        match $expr {
            Ok(value) => value,
            Err(err) => return Ok(Err(crate::TopErrors::clone(err))),
        }
    };
}
