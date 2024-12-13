mod find;
mod find_error;
mod resolve_type;

use super::{
    error::{ResolveError, ResolveErrorKind},
    expr::ResolveExprCtx,
};
use crate::{
    ast,
    name::ResolvedName,
    resolved::{self, Constraint},
    workspace::fs::FsNodeId,
};
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

pub fn resolve_constraints(constraints: &[ast::Type]) -> Result<Vec<Constraint>, ResolveError> {
    let mut resolved_constraints = vec![];

    for constraint in constraints {
        if let ast::TypeKind::Named(name, arguments) = &constraint.kind {
            resolved_constraints.push(match name.as_plain_str() {
                Some("PrimitiveAdd") if arguments.is_empty() => Constraint::PrimitiveAdd,
                _ => {
                    return Err(
                        ResolveErrorKind::UndeclaredTrait(name.to_string()).at(constraint.source)
                    )
                }
            });
        }
    }

    Ok(resolved_constraints)
}
