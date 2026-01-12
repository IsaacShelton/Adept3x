use crate::{ByteUnits, impl_math_for, impl_units_from};
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct BitUnits {
    units: u64,
}

impl BitUnits {
    pub const fn of(value: u64) -> Self {
        Self { units: value }
    }

    pub const fn bits(&self) -> u64 {
        self.units
    }

    pub fn min_unsigned(self) -> Result<u64, ()> {
        Ok(0)
    }

    pub fn max_unsigned(self) -> Result<u64, ()> {
        if self.units == 64 {
            Ok(u64::MAX)
        } else if self.units < 64 {
            Ok((1u64 << self.units) - 1)
        } else {
            Err(())
        }
    }

    pub fn min_signed(self) -> Result<i64, ()> {
        if self.units == 0 {
            Ok(0)
        } else {
            Ok(-(BitUnits::of(self.units - 1).max_unsigned()? as i64) - 1)
        }
    }

    pub fn max_signed(self) -> Result<i64, ()> {
        if self.units == 0 {
            Ok(0)
        } else {
            Ok(BitUnits::of(self.units - 1).max_unsigned()? as i64)
        }
    }
}

impl_math_for!(BitUnits);
impl_units_from!(BitUnits, u8);
impl_units_from!(BitUnits, u16);
impl_units_from!(BitUnits, u32);
impl_units_from!(BitUnits, u64);

impl From<ByteUnits> for BitUnits {
    fn from(value: ByteUnits) -> Self {
        Self {
            units: value.bytes() * 8,
        }
    }
}
