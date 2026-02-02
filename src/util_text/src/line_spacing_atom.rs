use serde_derive::{Deserialize, Serialize};
use std::fmt::Display;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LineSpacingAtom {
    pub count: usize,
}

impl Display for LineSpacingAtom {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for _ in 0..self.count {
            write!(f, "\n")?;
        }
        Ok(())
    }
}
