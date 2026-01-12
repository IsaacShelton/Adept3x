/*
    =======================  util_data_unit/src/lib.rs  =======================
    Measurement units for data
    ---------------------------------------------------------------------------
*/

mod atomic_byte_units;
mod bit_units;
mod byte_units;

pub use atomic_byte_units::AtomicByteUnits;
pub use bit_units::BitUnits;
pub use byte_units::ByteUnits;

#[macro_export]
macro_rules! implies {
    ($x:expr, $y:expr) => {
        !($x) || ($y)
    };
}

#[macro_export]
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

#[macro_export]
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

        impl ::std::ops::Add<$units> for $units {
            type Output = $units;

            fn add(self, rhs: $units) -> Self::Output {
                Self {
                    units: self.units + rhs.units,
                }
            }
        }

        impl ::std::ops::AddAssign<$units> for $units {
            fn add_assign(&mut self, rhs: $units) {
                self.units += rhs.units
            }
        }

        impl ::std::ops::Sub<$units> for $units {
            type Output = $units;

            fn sub(self, rhs: $units) -> Self::Output {
                Self {
                    units: self.units - rhs.units,
                }
            }
        }

        impl ::std::ops::SubAssign<$units> for $units {
            fn sub_assign(&mut self, rhs: $units) {
                self.units -= rhs.units
            }
        }

        impl ::std::ops::Mul<$units> for $units {
            type Output = $units;

            fn mul(self, rhs: $units) -> Self::Output {
                Self {
                    units: self.units * rhs.units,
                }
            }
        }

        impl ::std::ops::Mul<u32> for $units {
            type Output = $units;

            fn mul(self, rhs: u32) -> Self::Output {
                Self {
                    units: self.units * rhs as u64,
                }
            }
        }

        impl ::std::ops::Mul<u64> for $units {
            type Output = $units;

            fn mul(self, rhs: u64) -> Self::Output {
                Self {
                    units: self.units * rhs,
                }
            }
        }

        impl ::std::ops::Div<$units> for $units {
            type Output = u64;

            fn div(self, rhs: $units) -> Self::Output {
                self.units / rhs.units
            }
        }

        impl ::std::ops::Rem<$units> for $units {
            type Output = $units;

            fn rem(self, rhs: $units) -> Self::Output {
                Self {
                    units: self.units % rhs.units,
                }
            }
        }
    };
}
