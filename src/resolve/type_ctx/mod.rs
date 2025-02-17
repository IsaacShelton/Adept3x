mod find;
mod find_error;
mod resolve_type;

use super::expr::ResolveExprCtx;
use crate::{
    asg::{self, Asg},
    name::ResolvedName,
    workspace::fs::FsNodeId,
};
pub use resolve_type::ResolveTypeOptions;
use std::collections::{HashMap, HashSet};

#[derive(Debug)]
pub struct ResolveTypeCtx<'a> {
    asg: &'a Asg<'a>,
    module_fs_node_id: FsNodeId,
    file_fs_node_id: FsNodeId,
    types_in_modules: &'a HashMap<FsNodeId, HashMap<String, asg::TypeDecl>>,
    used_aliases_stack: HashSet<ResolvedName>,
}

impl<'a> ResolveTypeCtx<'a> {
    pub fn new(
        asg: &'a Asg,
        module_fs_node_id: FsNodeId,
        file_fs_node_id: FsNodeId,
        types_in_modules: &'a HashMap<FsNodeId, HashMap<String, asg::TypeDecl>>,
    ) -> Self {
        Self {
            asg,
            module_fs_node_id,
            file_fs_node_id,
            types_in_modules,
            used_aliases_stack: Default::default(),
        }
    }
}

impl<'a, 'b, 'c> From<&'c ResolveExprCtx<'a, 'b>> for ResolveTypeCtx<'c> {
    fn from(ctx: &'c ResolveExprCtx<'a, 'b>) -> Self {
        Self::new(
            ctx.asg,
            ctx.module_fs_node_id,
            ctx.physical_fs_node_id,
            ctx.types_in_modules,
        )
    }
}
