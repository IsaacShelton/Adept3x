use crate::{bit_units::BitUnits, impl_math_for, impl_units_from};
use derive_more::Sum;
use serde::{Deserialize, Serialize};

#[derive(
    Copy, Clone, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize, Sum,
)]
pub struct ByteUnits {
    units: u64,
}

impl ByteUnits {
    pub const ZERO: Self = Self { units: 0 };

    pub const fn of(value: u64) -> Self {
        Self { units: value }
    }

    pub const fn bytes(&self) -> u64 {
        self.units
    }

    pub fn alignment_at_offset(&self, offset: &Self) -> Self {
        // The largest power-of-2 common divisor of our alignment and the incoming offset
        let bits_in_either = self.units | offset.units;
        Self::of(bits_in_either & (!bits_in_either).wrapping_add(1))
    }

    pub fn to_bits(self) -> BitUnits {
        BitUnits::from(self)
    }

    pub fn from_bits(bits: BitUnits) -> ByteUnits {
        let bits = bits.bits();

        if bits % 8 == 0 {
            ByteUnits::of(bits / 8)
        } else {
            ByteUnits::of(bits / 8 + 1)
        }
    }

    pub fn next_power_of_two(self) -> Self {
        Self::of(self.units.next_power_of_two())
    }

    pub fn min_max_unsigned(self) -> Option<(u64, u64)> {
        self.min_unsigned()
            .and_then(|min| self.max_unsigned().map(|max| (min, max)))
            .ok()
    }

    pub fn min_max_signed(self) -> Option<(i64, i64)> {
        self.min_signed()
            .and_then(|min| self.max_signed().map(|max| (min, max)))
            .ok()
    }

    pub fn min_unsigned(self) -> Result<u64, ()> {
        self.to_bits().min_unsigned()
    }

    pub fn max_unsigned(self) -> Result<u64, ()> {
        self.to_bits().max_unsigned()
    }

    pub fn min_signed(self) -> Result<i64, ()> {
        self.to_bits().min_signed()
    }

    pub fn max_signed(self) -> Result<i64, ()> {
        self.to_bits().max_signed()
    }
}

impl_math_for!(ByteUnits);
impl_units_from!(ByteUnits, u8);
impl_units_from!(ByteUnits, u16);
impl_units_from!(ByteUnits, u32);
impl_units_from!(ByteUnits, u64);

impl TryFrom<BitUnits> for ByteUnits {
    type Error = ();

    fn try_from(value: BitUnits) -> Result<Self, ()> {
        if value.bits() % 8 == 0 {
            Ok(Self {
                units: value.bits() / 8,
            })
        } else {
            Err(())
        }
    }
}

impl TryFrom<usize> for ByteUnits {
    type Error = ();

    fn try_from(value: usize) -> Result<Self, ()> {
        value.try_into().map(|units| Self { units }).map_err(|_| ())
    }
}
