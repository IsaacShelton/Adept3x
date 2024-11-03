mod prepare;
mod resolve;

use super::{ctx::ResolveCtx, error::ResolveError};
use crate::{
    ast::AstWorkspace,
    resolved::{self},
};
use prepare::prepare_type_jobs;
use resolve::resolve_type_jobs;

pub fn resolve_type_definitions(
    ctx: &mut ResolveCtx,
    resolved_ast: &mut resolved::Ast,
    ast_workspace: &AstWorkspace,
) -> Result<(), ResolveError> {
    prepare_type_jobs(ctx, resolved_ast, ast_workspace).and_then(|type_jobs| {
        resolve_type_jobs(ctx, resolved_ast, ast_workspace, type_jobs.as_slice())
    })
}
