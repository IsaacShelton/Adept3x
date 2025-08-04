use derive_more::IsVariant;

// NOTE: Privacy is ordered from least private to most private
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, IsVariant)]
pub enum Privacy {
    Public,
    #[default]
    Protected,
    Private,
}
