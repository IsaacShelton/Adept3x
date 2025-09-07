use std::ops::{Add, AddAssign, Sub, SubAssign};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct RegCount {
    int: u32,
    sse: u32,
    is_unlimited: bool,
}

impl RegCount {
    pub fn zeros() -> Self {
        Self {
            int: 0,
            sse: 0,
            is_unlimited: false,
        }
    }

    pub fn ints(int: u32) -> Self {
        Self {
            int,
            sse: 0,
            is_unlimited: false,
        }
    }

    pub fn sses(sse: u32) -> Self {
        Self {
            int: 0,
            sse,
            is_unlimited: false,
        }
    }

    pub fn unlimited() -> Self {
        Self {
            int: u32::MAX,
            sse: u32::MAX,
            is_unlimited: true,
        }
    }

    pub fn can_spare(self, other: RegCount) -> bool {
        (self.int >= other.int && self.sse >= other.sse) || self.is_unlimited
    }

    pub fn has_ints(self, amount: u32) -> bool {
        self.int >= amount || self.is_unlimited
    }

    pub fn has_sses(self, amount: u32) -> bool {
        self.sse >= amount || self.is_unlimited
    }
}

impl Add for RegCount {
    type Output = RegCount;

    fn add(self, rhs: Self) -> Self::Output {
        if self.is_unlimited {
            self
        } else if rhs.is_unlimited {
            rhs
        } else {
            RegCount {
                int: self.int + rhs.int,
                sse: self.sse + rhs.sse,
                is_unlimited: false,
            }
        }
    }
}

impl Sub for RegCount {
    type Output = RegCount;

    fn sub(self, rhs: Self) -> Self::Output {
        if self.is_unlimited {
            self
        } else {
            RegCount {
                int: self.int - rhs.int,
                sse: self.sse - rhs.sse,
                is_unlimited: false,
            }
        }
    }
}

impl AddAssign for RegCount {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl SubAssign for RegCount {
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}
