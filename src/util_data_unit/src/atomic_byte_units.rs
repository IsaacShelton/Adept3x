use crate::byte_units::ByteUnits;
use std::sync::atomic::{self, AtomicU64};

#[derive(Debug, Default)]
pub struct AtomicByteUnits {
    units: AtomicU64,
}

impl AtomicByteUnits {
    pub const ZERO: Self = Self {
        units: AtomicU64::new(0),
    };

    pub const fn of(value: u64) -> Self {
        Self {
            units: AtomicU64::new(value),
        }
    }

    pub fn max(&self, other: ByteUnits, ordering: atomic::Ordering) {
        self.units.fetch_max(other.bytes(), ordering);
    }

    pub fn load(&self, ordering: atomic::Ordering) -> ByteUnits {
        ByteUnits::of(self.units.load(ordering))
    }

    pub fn into_inner(self) -> ByteUnits {
        ByteUnits::of(self.units.into_inner())
    }
}
