use crate::{Assumption, Assumptions};
use std::fmt::Debug;

/// Presumed is a monad that tracks the context of which
/// assumptions are necessary for a conclusion.
///
/// If a value is presumed, then it might be non-deterministic
/// what the value actually is, but it will be *some* valid value
/// from one of the many possible valid executions.
/// We can't rely on *which* valid value it will be unless
/// the assumptions have been cleared, which guarantees that the
/// enclosed value is the *only* valid value.
#[derive(Clone)]
pub struct Presumed<T> {
    assumptions: Assumptions,
    presumed: T,
}

impl Presumed<()> {
    pub fn assumed(assumption: Assumption) -> Presumed<()> {
        Presumed {
            assumptions: Assumptions::new(assumption),
            presumed: (),
        }
    }
}

impl<T> Presumed<T> {
    pub fn input(value: T) -> Self {
        Self {
            assumptions: Assumptions::empty(),
            presumed: value,
        }
    }

    pub fn and_then<S>(self, mut f: impl FnMut(T) -> Presumed<S>) -> Presumed<S> {
        let s = f(self.presumed);

        Presumed {
            presumed: s.presumed,
            assumptions: self.assumptions.concat(s.assumptions),
        }
    }

    pub fn unwrap_no_assumptions(self) -> T {
        if !self.assumptions.is_empty() {
            panic!("unwrap_no_assumptions() failed!");
        }
        self.presumed
    }

    pub fn conclusion(self) -> T {
        self.presumed
    }

    pub fn assumptions(&self) -> &Assumptions {
        &self.assumptions
    }

    pub fn as_ref(&self) -> Presumed<&T> {
        Presumed {
            assumptions: self.assumptions.clone(),
            presumed: &self.presumed,
        }
    }
}

impl<T: Debug> Debug for Presumed<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Presumed({:?})", self.presumed)
    }
}

impl<T, COLLECTION: FromIterator<T>> FromIterator<Presumed<T>> for Presumed<COLLECTION> {
    fn from_iter<I: IntoIterator<Item = Presumed<T>>>(iter: I) -> Presumed<COLLECTION> {
        let iter = iter.into_iter();
        let mut assumptions = Assumptions::empty();

        let collection = iter
            .map(|presumed| {
                assumptions = assumptions.concat(presumed.assumptions);
                presumed.presumed
            })
            .collect();

        Presumed {
            assumptions,
            presumed: collection,
        }
    }
}
