mod collect_polymorphs;
mod conform;
mod core_struct_info;
mod ctx;
mod destination;
mod error;
mod expr;
mod func_body;
mod func_haystack;
mod func_head;
mod global_variable;
mod helper_expr;
mod impl_head;
mod initialized;
mod job;
mod polymorph;
mod stmt;
mod type_ctx;
mod type_definition;
mod unify_types;
mod variable_haystack;

use self::error::ResolveError;
use crate::{asg::Asg, ast::AstWorkspace, cli::BuildOptions};
use ctx::ResolveCtx;
use func_body::resolve_func_bodies;
use func_head::create_func_heads;
use global_variable::resolve_global_variables;
use helper_expr::resolve_helper_expressions;
use initialized::Initialized;
pub use polymorph::*;
pub use stmt::resolve_stmts;
use type_ctx::ResolveTypeCtx;
use type_definition::resolve_type_definitions;

pub fn resolve<'a>(
    workspace: &'a AstWorkspace,
    options: &BuildOptions,
) -> Result<Asg<'a>, ResolveError> {
    let mut ctx = ResolveCtx::new();
    let source_files = workspace.source_files;
    let mut asg = Asg::new(source_files, &workspace);

    resolve_type_definitions(&mut ctx, &mut asg, workspace)?;
    resolve_global_variables(&mut ctx, &mut asg, workspace)?;
    create_func_heads(&mut ctx, &mut asg, workspace, options)?;
    resolve_helper_expressions(&mut ctx, &mut asg, workspace)?;
    resolve_func_bodies(&mut ctx, &mut asg, workspace)?;

    Ok(asg)
}
