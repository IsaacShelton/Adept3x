use crate::IsDiv;

pub trait Minor {
    fn minor(self) -> Self;
}

impl Minor for u32 {
    fn minor(self) -> Self {
        self + 1
    }
}

impl Minor for Rev {
    fn minor(self) -> Self {
        Self {
            major: self.major,
            minor: self.minor + 1,
        }
    }
}

pub trait Major {
    fn major(self) -> Self;
}

impl Major for Rev {
    fn major(self) -> Self {
        Self {
            major: self.major + 1,
            minor: 0,
        }
    }
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct Rev {
    major: u32,
    minor: u32,
}

impl Ord for Rev {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.major
            .cmp(&other.major)
            .then(self.minor.cmp(&other.minor))
    }
}

impl PartialOrd for Rev {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl IsDiv for Rev {
    fn is_div(&self, n: u32) -> bool {
        self.minor >= n
    }
}

impl IsDiv for u32 {
    fn is_div(&self, n: u32) -> bool {
        *self >= n
    }
}
