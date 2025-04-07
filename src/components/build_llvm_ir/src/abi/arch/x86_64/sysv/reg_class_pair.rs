use super::reg_class::RegClass;

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct RegClassPair {
    pub high: RegClass,
    pub low: RegClass,
}

impl RegClassPair {
    pub fn merge_with(&mut self, other: RegClassPair) {
        self.high = self.high.merge(other.high);
        self.low = self.low.merge(other.low);
    }
}
