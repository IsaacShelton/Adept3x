use super::{
    conform_expr,
    error::{ResolveError, ResolveErrorKind},
    expr::{resolve_expr, ResolveExprCtx},
    resolve_expr_to_destination, resolve_type, Initialized,
};
use crate::{ast, resolved};

pub fn resolve_stmts(
    ctx: &mut ResolveExprCtx<'_, '_>,
    stmts: &[ast::Stmt],
) -> Result<Vec<resolved::Stmt>, ResolveError> {
    let mut resolved_stmts = Vec::with_capacity(stmts.len());

    for stmt in stmts.iter() {
        resolved_stmts.push(resolve_stmt(ctx, stmt)?);
    }

    Ok(resolved_stmts)
}

pub fn resolve_stmt<'a>(
    ctx: &mut ResolveExprCtx<'_, '_>,
    ast_stmt: &ast::Stmt,
) -> Result<resolved::Stmt, ResolveError> {
    let source = ast_stmt.source;

    match &ast_stmt.kind {
        ast::StmtKind::Return(value) => {
            let return_value = if let Some(value) = value {
                let result = resolve_expr(ctx, value, Initialized::Require)?;

                let function = ctx
                    .resolved_ast
                    .functions
                    .get(ctx.resolved_function_ref)
                    .unwrap();

                if let Some(result) = conform_expr(&result, &function.return_type) {
                    Some(result.expr)
                } else {
                    return Err(ResolveError::new(
                        ctx.resolved_ast.source_file_cache,
                        source,
                        ResolveErrorKind::CannotReturnValueOfType {
                            returning: result.resolved_type.to_string(),
                            expected: function.return_type.to_string(),
                        },
                    ));
                }
            } else {
                let function = ctx
                    .resolved_ast
                    .functions
                    .get(ctx.resolved_function_ref)
                    .unwrap();

                if function.return_type != resolved::Type::Void {
                    return Err(ResolveError::new(
                        ctx.resolved_ast.source_file_cache,
                        source,
                        ResolveErrorKind::CannotReturnVoid {
                            expected: function.return_type.to_string(),
                        },
                    ));
                }

                None
            };

            Ok(resolved::Stmt::new(
                resolved::StmtKind::Return(return_value),
                source,
            ))
        }
        ast::StmtKind::Expr(value) => Ok(resolved::Stmt::new(
            resolved::StmtKind::Expr(resolve_expr(ctx, value, Initialized::Require)?),
            source,
        )),
        ast::StmtKind::Declaration(declaration) => {
            let resolved_type = resolve_type(
                ctx.type_search_ctx,
                ctx.resolved_ast.source_file_cache,
                &declaration.ast_type,
            )?;

            let value = declaration
                .value
                .as_ref()
                .map(|value| resolve_expr(ctx, value, Initialized::Require))
                .transpose()?
                .as_ref()
                .map(|value| match conform_expr(value, &resolved_type) {
                    Some(value) => Ok(value.expr),
                    None => Err(ResolveError::new(
                        ctx.resolved_ast.source_file_cache,
                        source,
                        ResolveErrorKind::CannotAssignValueOfType {
                            from: value.resolved_type.to_string(),
                            to: resolved_type.to_string(),
                        },
                    )),
                })
                .transpose()?;

            let function = ctx
                .resolved_ast
                .functions
                .get_mut(ctx.resolved_function_ref)
                .unwrap();

            let key = function
                .variables
                .add_variable(resolved_type.clone(), value.is_some());

            ctx.variable_search_ctx
                .put(&declaration.name, resolved_type.clone(), key);

            Ok(resolved::Stmt::new(
                resolved::StmtKind::Declaration(resolved::Declaration { key, value }),
                source,
            ))
        }
        ast::StmtKind::Assignment(assignment) => {
            let destination_expr = resolve_expr(
                ctx,
                &assignment.destination,
                Initialized::AllowUninitialized,
            )?;

            let value = resolve_expr(ctx, &assignment.value, Initialized::Require)?;

            let value = conform_expr(&value, &destination_expr.resolved_type).ok_or_else(|| {
                ResolveError::new(
                    ctx.resolved_ast.source_file_cache,
                    source,
                    ResolveErrorKind::CannotAssignValueOfType {
                        from: value.resolved_type.to_string(),
                        to: destination_expr.resolved_type.to_string(),
                    },
                )
            })?;

            let destination = resolve_expr_to_destination(
                ctx.resolved_ast.source_file_cache,
                destination_expr.expr,
            )?;

            // Mark destination as initialized
            match &destination.kind {
                resolved::DestinationKind::Variable(variable) => {
                    let function = ctx
                        .resolved_ast
                        .functions
                        .get_mut(ctx.resolved_function_ref)
                        .unwrap();

                    function
                        .variables
                        .get(variable.key)
                        .expect("variable being assigned to exists")
                        .set_initialized();
                }
                resolved::DestinationKind::GlobalVariable(..) => (),
                resolved::DestinationKind::Member(..) => (),
            }

            Ok(resolved::Stmt::new(
                resolved::StmtKind::Assignment(resolved::Assignment {
                    destination,
                    value: value.expr,
                }),
                source,
            ))
        }
    }
}
