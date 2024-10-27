use super::{FloatOrSign, FloatOrSignLax};

#[derive(Copy, Clone, Debug)]
pub enum FloatOrInteger {
    Integer,
    Float,
}

impl From<FloatOrSignLax> for FloatOrInteger {
    fn from(value: FloatOrSignLax) -> Self {
        match value {
            FloatOrSignLax::Integer(_) | FloatOrSignLax::IndeterminateInteger(_) => Self::Integer,
            FloatOrSignLax::Float => Self::Float,
        }
    }
}

impl From<FloatOrSign> for FloatOrInteger {
    fn from(value: FloatOrSign) -> Self {
        match value {
            FloatOrSign::Integer(_) => Self::Integer,
            FloatOrSign::Float => Self::Float,
        }
    }
}
