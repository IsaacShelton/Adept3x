use crate::{data_units::BitUnits, target::Target};
use derive_more::IsVariant;
use std::cmp::Ordering;

#[derive(Copy, Clone, Debug)]
pub enum IntegerRank {
    Bool,
    Char,
    Short,
    Int,
    Long,
    LongLong,
    FixedInt(BitUnits),
}

impl IntegerRank {
    pub fn compare_for_target(left: Self, right: Self, target: &Target) -> Ordering {
        let left_precision = left.precision(target);
        let right_precision = right.precision(target);
        left_precision.cmp(&right_precision)
    }

    pub fn precision(&self, target: &Target) -> IntegerPrecision {
        match self {
            IntegerRank::Bool => IntegerPrecision::boolean(),
            IntegerRank::Char => IntegerPrecision::flexible(target.char_layout().width.to_bits()),
            IntegerRank::Short => IntegerPrecision::flexible(target.short_layout().width.to_bits()),
            IntegerRank::Int => IntegerPrecision::flexible(target.int_layout().width.to_bits()),
            IntegerRank::Long => IntegerPrecision::flexible(target.long_layout().width.to_bits()),
            IntegerRank::LongLong => {
                IntegerPrecision::flexible(target.longlong_layout().width.to_bits())
            }
            IntegerRank::FixedInt(bits) => IntegerPrecision::fixed(*bits),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, IsVariant)]
pub enum IntegerPrecision {
    Boolean,
    Normal { bits: BitUnits, flexible: bool },
}

impl IntegerPrecision {
    pub fn boolean() -> Self {
        Self::Boolean
    }
    pub fn flexible(bits: BitUnits) -> Self {
        Self::Normal {
            bits,
            flexible: true,
        }
    }

    pub fn fixed(bits: BitUnits) -> Self {
        Self::Normal {
            bits,
            flexible: false,
        }
    }
}

impl Ord for IntegerPrecision {
    fn cmp(&self, other: &Self) -> Ordering {
        match self {
            IntegerPrecision::Boolean => other
                .is_normal()
                .then_some(Ordering::Less)
                .unwrap_or(Ordering::Equal),
            IntegerPrecision::Normal { bits, flexible } => {
                let IntegerPrecision::Normal {
                    bits: other_bits,
                    flexible: other_flexible,
                } = other
                else {
                    return Ordering::Greater;
                };

                bits.cmp(other_bits)
                    .then_with(|| flexible.cmp(other_flexible))
            }
        }
    }
}
