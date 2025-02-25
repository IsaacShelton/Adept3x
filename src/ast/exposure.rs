use derive_more::IsVariant;

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, IsVariant)]
pub enum Exposure {
    #[default]
    Hidden,
    Exposed,
}
