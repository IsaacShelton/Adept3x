use crate::InstrRef;
use derive_more::From;
use std::fmt::Display;

// From the perspective of before type checking, this is either:
// * Known void
// * Known never (referencing end instruction)
// * From an instruction (although this does not rule out the possiblity that it's void or never)
//   e.g. A call instruction could result in either of these.
#[derive(Copy, Clone, Debug, From)]
pub enum CfgValue {
    Void,
    Instr(InstrRef),
}

impl CfgValue {
    pub fn is_known_void(&self) -> bool {
        matches!(self, Self::Void)
    }
}

impl Display for CfgValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CfgValue::Void => write!(f, "<void>"),
            CfgValue::Instr(instr_ref) => instr_ref.fmt(f),
        }
    }
}
