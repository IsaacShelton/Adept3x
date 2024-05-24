use super::Inflow;

pub trait TryPeek<T, E: Clone>: Inflow<Result<T, E>> {
    fn try_peek<'a>(&'a mut self) -> Result<&'a T, E>
    where
        E: 'a,
    {
        match self.peek().as_ref() {
            Ok(reference) => Ok(reference),
            Err(error) => Err(error.clone()),
        }
    }
}

impl<T, E: Clone, I: Inflow<Result<T, E>>> TryPeek<T, E> for I {}
