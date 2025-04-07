use derive_more::IsVariant;

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, IsVariant)]
pub enum Exposure {
    #[default]
    Hidden,
    Exposed,
}

#[derive(Copy, Clone, Debug, IsVariant)]
pub enum SymbolOwnership {
    Reference,
    Owned(Exposure),
}

impl SymbolOwnership {
    pub fn from_foreign_and_exposed(is_foreign: bool, is_exposed: bool) -> Self {
        if is_exposed {
            Self::Owned(Exposure::Exposed)
        } else if is_foreign {
            Self::Reference
        } else {
            Self::Owned(Exposure::Hidden)
        }
    }

    pub fn should_mangle(&self) -> bool {
        !matches!(self, Self::Reference | Self::Owned(Exposure::Exposed))
    }
}

impl Default for SymbolOwnership {
    fn default() -> Self {
        Self::Owned(Exposure::Hidden)
    }
}
