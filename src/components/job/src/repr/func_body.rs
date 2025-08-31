use super::Variables;
use crate::Cfg;

#[derive(Clone, Debug)]
pub struct FuncBody<'env> {
    pub cfg: &'env Cfg<'env>,
    pub variables: Variables<'env>,
}
