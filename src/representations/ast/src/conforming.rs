use primitives::CIntegerAssumptions;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
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

    pub fn auto_c_integer_to_bool_conversion(&self) -> bool {
        match self {
            ConformBehavior::Adept(_) => false,
            ConformBehavior::C => true,
        }
    }
}
