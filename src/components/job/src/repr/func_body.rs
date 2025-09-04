use super::Variables;
use crate::{BasicBlockId, Cfg};

#[derive(Clone, Debug)]
pub struct FuncBody<'env> {
    pub cfg: &'env Cfg<'env>,
    pub post_order: &'env [BasicBlockId],
    pub variables: Variables<'env>,
}
