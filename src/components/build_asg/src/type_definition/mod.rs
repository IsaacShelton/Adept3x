mod prepare;
mod resolve;

use super::{ctx::ResolveCtx, error::ResolveError};
use asg::Asg;
use ast_workspace::AstWorkspace;
use prepare::prepare_type_jobs;
use resolve::resolve_type_jobs;

pub fn resolve_type_definitions(
    ctx: &mut ResolveCtx,
    asg: &mut Asg,
    ast_workspace: &AstWorkspace,
) -> Result<(), ResolveError> {
    prepare_type_jobs(ctx, asg, ast_workspace)
        .and_then(|type_jobs| resolve_type_jobs(ctx, asg, ast_workspace, type_jobs.as_slice()))
}
