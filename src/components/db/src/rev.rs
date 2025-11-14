#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct NumberedRevision(u32);

impl NumberedRevision {
    pub fn succ(&self) -> NumberedRevision {
        Self(self.0 + 1)
    }
}
