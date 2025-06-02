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
            Self::Adept(assumptions) => *assumptions,
            Self::C => Default::default(),
        }
    }

    pub fn auto_c_integer_to_bool_conversion(&self) -> bool {
        match self {
            Self::Adept(_) => false,
            Self::C => true,
        }
    }

    pub fn language(&self) -> Language {
        match self {
            ConformBehavior::Adept(_) => Language::Adept,
            ConformBehavior::C => Language::C,
        }
    }
}
