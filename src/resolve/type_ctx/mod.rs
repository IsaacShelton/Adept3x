mod find;
mod find_error;
mod resolve_type;

use super::expr::ResolveExprCtx;
use crate::{name::ResolvedName, resolved, workspace::fs::FsNodeId};
use std::collections::{HashMap, HashSet};

#[derive(Debug)]
pub struct ResolveTypeCtx<'a> {
    resolved_ast: &'a resolved::Ast<'a>,
    module_fs_node_id: FsNodeId,
    file_fs_node_id: FsNodeId,
    types_in_modules: &'a HashMap<FsNodeId, HashMap<String, resolved::TypeDecl>>,
    used_aliases_stack: HashSet<ResolvedName>,
}

impl<'a> ResolveTypeCtx<'a> {
    pub fn new(
        resolved_ast: &'a resolved::Ast,
        module_fs_node_id: FsNodeId,
        file_fs_node_id: FsNodeId,
        types_in_modules: &'a HashMap<FsNodeId, HashMap<String, resolved::TypeDecl>>,
    ) -> Self {
        Self {
            resolved_ast,
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
            ctx.resolved_ast,
            ctx.module_fs_node_id,
            ctx.physical_fs_node_id,
            ctx.types_in_modules,
        )
    }
}
