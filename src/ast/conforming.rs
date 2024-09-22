use super::CIntegerAssumptions;

#[derive(Copy, Clone, Debug)]
pub enum Language {
    Adept,
    C,
}

#[derive(Copy, Clone, Debug)]
pub enum ConformBehavior {
    Adept(CIntegerAssumptions),
    C,
}

impl ConformBehavior {
    pub fn c_integer_assumptions(&self) -> CIntegerAssumptions {
        match self {
            ConformBehavior::Adept(assumptions) => *assumptions,
            ConformBehavior::C => Default::default(),
        }
    }
}
