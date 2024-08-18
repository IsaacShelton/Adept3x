use super::{TargetArch, TargetOs};
use std::fmt::Display;

pub trait IntoDisplay {
    fn display(self) -> impl Display;
}

impl IntoDisplay for Option<TargetArch> {
    fn display(self) -> impl Display {
        DisplayOrUnknown(self)
    }
}

impl IntoDisplay for Option<TargetOs> {
    fn display(self) -> impl Display {
        DisplayOrUnknown(self)
    }
}

struct DisplayOrUnknown<T: Display>(pub Option<T>);

impl<T: Display> Display for DisplayOrUnknown<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.0 {
            Some(inner) => inner.fmt(f),
            None => write!(f, "unknown"),
        }
    }
}
