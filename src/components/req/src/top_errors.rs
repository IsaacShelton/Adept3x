use crate::Error;
use std::fmt::Display;
use top_n::TopN;

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct TopErrors {
    top_n: TopN<Error>,
}

impl Default for TopErrors {
    fn default() -> Self {
        Self {
            top_n: TopN::new(1),
        }
    }
}

impl From<Error> for TopErrors {
    fn from(value: Error) -> Self {
        Self {
            top_n: TopN::from_iter(1, std::iter::once(value), |a, b| a.cmp(b)),
        }
    }
}

impl TopErrors {
    pub fn push(&mut self, error: Error) -> &mut Self {
        self.top_n.push(error, |a, b| a.cmp(b));
        self
    }
}

impl Display for TopErrors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<err msg>")
    }
}

impl<T, S> Into<Result<Result<T, TopErrors>, S>> for Error {
    fn into(self) -> Result<Result<T, TopErrors>, S> {
        Ok(Err(TopErrors::from(self)))
    }
}
