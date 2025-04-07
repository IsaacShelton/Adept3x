mod regular;
mod short_circuiting;

pub use regular::*;
pub use short_circuiting::*;

#[derive(Clone, Debug)]
pub enum BinaryOperator {
    Basic(BasicBinaryOperator),
    ShortCircuiting(ShortCircuitingBinaryOperator),
}

impl From<BasicBinaryOperator> for BinaryOperator {
    fn from(value: BasicBinaryOperator) -> Self {
        Self::Basic(value)
    }
}

impl From<ShortCircuitingBinaryOperator> for BinaryOperator {
    fn from(value: ShortCircuitingBinaryOperator) -> Self {
        Self::ShortCircuiting(value)
    }
}
