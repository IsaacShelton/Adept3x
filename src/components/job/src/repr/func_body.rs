use super::Variables;
use crate::{Resolved, cfg::NodeId};
use arena::ArenaMap;

#[derive(Clone, Debug)]
pub struct FuncBody<'env> {
    pub variables: Variables<'env>,
    pub resolved: ArenaMap<NodeId, Resolved<'env>>,
}
