use std::ops::{Add, AddAssign, Div, Mul, Rem, Sub, SubAssign};

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, Ord, PartialOrd)]
pub struct ByteUnits {
    units: u64,
}

impl ByteUnits {
    pub const fn of(value: u64) -> Self {
        Self { units: value }
    }

    pub const fn bytes(&self) -> u64 {
        self.units
    }
}

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, Ord, PartialOrd)]
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
}

macro_rules! impl_units_from {
    ($units:ty, $ty:ty) => {
        impl From<$ty> for $units {
            fn from(value: $ty) -> Self {
                Self {
                    units: value.into(),
                }
            }
        }
    };
}

impl_units_from!(ByteUnits, u8);
impl_units_from!(ByteUnits, u16);
impl_units_from!(ByteUnits, u32);
impl_units_from!(ByteUnits, u64);

impl_units_from!(BitUnits, u8);
impl_units_from!(BitUnits, u16);
impl_units_from!(BitUnits, u32);
impl_units_from!(BitUnits, u64);

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

impl From<ByteUnits> for BitUnits {
    fn from(value: ByteUnits) -> Self {
        Self {
            units: value.bytes() * 8,
        }
    }
}

macro_rules! impl_math_for {
    ($units:ty) => {
        impl $units {
            pub fn is_zero(&self) -> bool {
                self.units == 0
            }

            pub fn align_to(&self, align: $units) -> $units {
                let width = self.units;
                let align = align.units;

                assert_ne!(align, 0);
                Self::of((width + align - 1) & !(align - 1))
            }

            pub fn is_power_of_2(&self) -> bool {
                (self.units & (self.units - 1)) == 0
            }
        }

        impl Add<$units> for $units {
            type Output = $units;

            fn add(self, rhs: $units) -> Self::Output {
                Self {
                    units: self.units + rhs.units,
                }
            }
        }

        impl AddAssign<$units> for $units {
            fn add_assign(&mut self, rhs: $units) {
                self.units += rhs.units
            }
        }

        impl Sub<$units> for $units {
            type Output = $units;

            fn sub(self, rhs: $units) -> Self::Output {
                Self {
                    units: self.units - rhs.units,
                }
            }
        }

        impl SubAssign<$units> for $units {
            fn sub_assign(&mut self, rhs: $units) {
                self.units -= rhs.units
            }
        }

        impl Mul<$units> for $units {
            type Output = $units;

            fn mul(self, rhs: $units) -> Self::Output {
                Self {
                    units: self.units * rhs.units,
                }
            }
        }

        impl Mul<u64> for $units {
            type Output = $units;

            fn mul(self, rhs: u64) -> Self::Output {
                Self {
                    units: self.units * rhs,
                }
            }
        }

        impl Div<$units> for $units {
            type Output = u64;

            fn div(self, rhs: $units) -> Self::Output {
                self.units / rhs.units
            }
        }

        impl Rem<$units> for $units {
            type Output = $units;

            fn rem(self, rhs: $units) -> Self::Output {
                Self {
                    units: self.units % rhs.units,
                }
            }
        }
    };
}

impl_math_for!(ByteUnits);
impl_math_for!(BitUnits);
