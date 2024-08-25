#[derive(Copy, Clone, Debug, Default)]
pub enum ConformMode {
    #[default]
    Normal,
    ParameterPassing,
    Explicit,
}

impl ConformMode {
    pub fn allow_pointer_into_void_pointer(&self) -> bool {
        match self {
            Self::Normal => false,
            Self::ParameterPassing => true,
            Self::Explicit => true,
        }
    }

    pub fn allow_lossy_integer(&self) -> bool {
        matches!(self, Self::Explicit)
    }
}
