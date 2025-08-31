use derive_more::IsVariant;
use std::fmt::Display;

// NOTE: Privacy is ordered from least private to most private
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, IsVariant)]
pub enum Privacy {
    Public,
    #[default]
    Protected,
    Private,
}

impl Display for Privacy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Privacy::Public => write!(f, "public"),
            Privacy::Protected => write!(f, "protected"),
            Privacy::Private => write!(f, "private"),
        }
    }
}
