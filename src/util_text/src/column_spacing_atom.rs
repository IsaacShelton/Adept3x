use serde_derive::{Deserialize, Serialize};
use std::fmt::Display;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ColumnSpacingAtom {
    Spaces(u32),
    Tabs(u32),
}

impl ColumnSpacingAtom {
    pub fn len(&self) -> u32 {
        match self {
            ColumnSpacingAtom::Spaces(count) => *count,
            ColumnSpacingAtom::Tabs(count) => *count,
        }
    }
}

impl Display for ColumnSpacingAtom {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ColumnSpacingAtom::Spaces(count) => {
                for _ in 0..*count {
                    write!(f, " ")?;
                }
                Ok(())
            }
            ColumnSpacingAtom::Tabs(count) => {
                for _ in 0..*count {
                    write!(f, "\t")?;
                }
                Ok(())
            }
        }
    }
}
