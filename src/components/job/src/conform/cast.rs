use crate::repr::UnaliasedType;
use num_bigint::BigInt;
use ordered_float::NotNan;
use primitives::IntegerSign;

#[derive(Clone, Debug)]
pub enum UnaryCast<'env> {
    SpecializeBoolean(bool),
    SpecializeInteger(&'env BigInt),
    SpecializeFloat(Option<NotNan<f64>>),
    SpecializePointerOuter(UnaliasedType<'env>),
    SpecializeAsciiChar(u8),
    Dereference {
        after_deref: UnaliasedType<'env>,
        then: Option<&'env UnaryCast<'env>>,
    },
    Extend(IntegerSign),
    Truncate,
}

impl<'env> UnaryCast<'env> {
    pub fn is_dereference(&self) -> bool {
        matches!(self, Self::Dereference { .. })
    }
}
