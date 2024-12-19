mod conform;
mod core_structure_info;
mod ctx;
mod destination;
mod error;
mod expr;
mod function_body;
mod function_haystack;
mod function_head;
mod global_variable;
mod helper_expr;
mod initialized;
mod job;
mod polymorph;
mod stmt;
mod type_ctx;
mod type_definition;
mod unify_types;
mod variable_haystack;

use self::error::ResolveError;
use crate::{
    ast::AstWorkspace,
    cli::BuildOptions,
    resolved::{self, Implementations},
};
use ctx::ResolveCtx;
use function_body::resolve_function_bodies;
use function_head::create_function_heads;
use global_variable::resolve_global_variables;
use helper_expr::resolve_helper_expressions;
use initialized::Initialized;
pub use polymorph::*;
pub use stmt::resolve_stmts;
use type_ctx::ResolveTypeCtx;
use type_definition::resolve_type_definitions;

pub fn resolve<'a>(
    ast_workspace: &'a AstWorkspace,
    implementations: &'a Implementations,
    options: &BuildOptions,
) -> Result<resolved::Ast<'a>, ResolveError> {
    let mut ctx = ResolveCtx::new(implementations);
    let source_files = ast_workspace.source_files;
    let mut resolved_ast = resolved::Ast::new(source_files, &ast_workspace);

    resolve_type_definitions(&mut ctx, &mut resolved_ast, ast_workspace)?;
    resolve_global_variables(&mut ctx, &mut resolved_ast, ast_workspace)?;
    create_function_heads(&mut ctx, &mut resolved_ast, ast_workspace, options)?;
    resolve_helper_expressions(&mut ctx, &mut resolved_ast, ast_workspace)?;
    resolve_function_bodies(&mut ctx, &mut resolved_ast, ast_workspace)?;

    Ok(resolved_ast)
}
