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
    resolved::{self, Constraint, CurrentConstraints},
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
    current_constraints: &'a CurrentConstraints,
}

impl<'a> ResolveTypeCtx<'a> {
    pub fn new(
        resolved_ast: &'a resolved::Ast,
        module_fs_node_id: FsNodeId,
        file_fs_node_id: FsNodeId,
        types_in_modules: &'a HashMap<FsNodeId, HashMap<String, resolved::TypeDecl>>,
        current_constraints: &'a CurrentConstraints,
    ) -> Self {
        Self {
            resolved_ast,
            module_fs_node_id,
            file_fs_node_id,
            types_in_modules,
            used_aliases_stack: Default::default(),
            current_constraints,
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
            &ctx.current_constraints,
        )
    }
}

pub fn resolve_constraints(
    type_ctx: &ResolveTypeCtx,
    constraints: &[ast::Type],
) -> Result<Vec<Constraint>, ResolveError> {
    let mut resolved_constraints = vec![];

    for constraint in constraints {
        resolved_constraints.push(resolve_constraint(type_ctx, constraint)?);
    }

    Ok(resolved_constraints)
}

pub fn resolve_constraint(
    type_ctx: &ResolveTypeCtx,
    constraint: &ast::Type,
) -> Result<Constraint, ResolveError> {
    if let ast::TypeKind::Named(name, arguments) = &constraint.kind {
        match name.as_plain_str() {
            Some("PrimitiveAdd") if arguments.is_empty() => return Ok(Constraint::PrimitiveAdd),
            _ => {
                let resolved_type = type_ctx.resolve(constraint).map_err(|err| {
                    if let ResolveErrorKind::UndeclaredType { name } = err.kind {
                        ResolveErrorKind::UndeclaredTrait(name).at(err.source)
                    } else {
                        err
                    }
                })?;

                let resolved::TypeKind::Trait(_, trait_ref) = &resolved_type.kind else {
                    return Err(ResolveErrorKind::TypeIsNotATrait {
                        name: resolved_type.to_string(),
                    }
                    .at(resolved_type.source));
                };

                return Ok(Constraint::Trait(*trait_ref));
            }
        }
    }

    return Err(ResolveErrorKind::TypeIsNotATrait {
        name: constraint.to_string(),
    }
    .at(constraint.source));
}
