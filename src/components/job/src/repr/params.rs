use super::Type;

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Params<'env> {
    pub required: &'env [Param<'env>],
    pub is_cstyle_vararg: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Param<'env> {
    pub name: Option<&'env str>,
    pub ty: &'env Type<'env>,
}
