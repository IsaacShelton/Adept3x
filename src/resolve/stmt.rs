use super::{
    conform::{conform_expr, ConformMode, Perform},
    destination::resolve_expr_to_destination,
    error::{ResolveError, ResolveErrorKind},
    expr::{resolve_basic_binary_operator, resolve_expr, PreferredType, ResolveExprCtx},
    Initialized,
};
use crate::{ast, resolved};
use std::borrow::Cow;

pub fn resolve_stmts(
    ctx: &mut ResolveExprCtx,
    stmts: &[ast::Stmt],
) -> Result<Vec<resolved::Stmt>, ResolveError> {
    let mut resolved_stmts = Vec::with_capacity(stmts.len());

    for stmt in stmts.iter() {
        resolved_stmts.push(resolve_stmt(ctx, stmt)?);
    }

    Ok(resolved_stmts)
}

pub fn resolve_stmt(
    ctx: &mut ResolveExprCtx,
    ast_stmt: &ast::Stmt,
) -> Result<resolved::Stmt, ResolveError> {
    let source = ast_stmt.source;

    match &ast_stmt.kind {
        ast::StmtKind::Return(value) => {
            let Some(resolved_function_ref) = ctx.resolved_function_ref else {
                return Err(ResolveErrorKind::CannotReturnOutsideFunction.at(ast_stmt.source));
            };

            let return_value = if let Some(value) = value {
                let result = resolve_expr(
                    ctx,
                    value,
                    Some(PreferredType::ReturnType(resolved_function_ref)),
                    Initialized::Require,
                )?;

                let mut return_type = Cow::Borrowed(
                    &ctx.resolved_ast
                        .functions
                        .get(resolved_function_ref)
                        .unwrap()
                        .return_type,
                );

                if return_type.kind.contains_polymorph() {
                    let mut stripped = return_type.as_ref().clone();
                    stripped.strip_constraints();
                    return_type = Cow::Owned(stripped);
                }

                if let Ok(result) = conform_expr::<Perform>(
                    ctx,
                    &result,
                    &return_type,
                    ConformMode::Normal,
                    ctx.adept_conform_behavior(),
                    source,
                ) {
                    Some(result.expr)
                } else {
                    return Err(ResolveErrorKind::CannotReturnValueOfType {
                        returning: result.resolved_type.to_string(),
                        expected: return_type.to_string(),
                    }
                    .at(source));
                }
            } else {
                let function = ctx
                    .resolved_ast
                    .functions
                    .get(resolved_function_ref)
                    .unwrap();

                if function.return_type.kind != resolved::TypeKind::Void {
                    return Err(ResolveErrorKind::CannotReturnVoid {
                        expected: function.return_type.to_string(),
                    }
                    .at(source));
                }

                None
            };

            Ok(resolved::Stmt::new(
                resolved::StmtKind::Return(return_value),
                source,
            ))
        }
        ast::StmtKind::Expr(value) => Ok(resolved::Stmt::new(
            resolved::StmtKind::Expr(resolve_expr(ctx, value, None, Initialized::Require)?),
            source,
        )),
        ast::StmtKind::Declaration(declaration) => {
            let resolved_type = ctx.type_ctx().resolve(&declaration.ast_type)?;

            let value = declaration
                .initial_value
                .as_ref()
                .map(|value| {
                    resolve_expr(
                        ctx,
                        value,
                        Some(PreferredType::of(&resolved_type)),
                        Initialized::Require,
                    )
                })
                .transpose()?
                .as_ref()
                .map(|value| {
                    conform_expr::<Perform>(
                        ctx,
                        value,
                        &resolved_type,
                        ConformMode::Normal,
                        ctx.adept_conform_behavior(),
                        source,
                    )
                    .map(|value| value.expr)
                    .map_err(|_| {
                        ResolveErrorKind::CannotAssignValueOfType {
                            from: value.resolved_type.to_string(),
                            to: resolved_type.to_string(),
                        }
                        .at(source)
                    })
                })
                .transpose()?;

            // NOTE: Eventually, we could allow declaring variables without an initializer,
            // but doing so would require tracking initialization for all possible paths,
            // which is not pretty. For the time being, we will simply disallow this.
            // The real question is whether being able to is worth all of the complexity that it brings.
            if value.is_none() {
                return Err(ResolveErrorKind::MustInitializeVariable {
                    name: declaration.name.clone(),
                }
                .at(source));
            }

            let Some(resolved_function_ref) = ctx.resolved_function_ref else {
                return Err(
                    ResolveErrorKind::CannotDeclareVariableOutsideFunction.at(ast_stmt.source)
                );
            };

            let function = ctx
                .resolved_ast
                .functions
                .get_mut(resolved_function_ref)
                .unwrap();

            let key = function
                .variables
                .add_variable(resolved_type.clone(), value.is_some());

            ctx.variable_haystack
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
                None,
                Initialized::AllowUninitialized,
            )?;

            let value = resolve_expr(
                ctx,
                &assignment.value,
                Some(PreferredType::of(&destination_expr.resolved_type)),
                Initialized::Require,
            )?;

            let value = conform_expr::<Perform>(
                ctx,
                &value,
                &destination_expr.resolved_type,
                ConformMode::Normal,
                ctx.adept_conform_behavior(),
                source,
            )
            .map_err(|_| {
                ResolveErrorKind::CannotAssignValueOfType {
                    from: value.resolved_type.to_string(),
                    to: destination_expr.resolved_type.to_string(),
                }
                .at(source)
            })?;

            let destination = resolve_expr_to_destination(destination_expr)?;

            let Some(resolved_function_ref) = ctx.resolved_function_ref else {
                return Err(
                    ResolveErrorKind::CannotAssignVariableOutsideFunction.at(ast_stmt.source)
                );
            };

            // Mark destination as initialized
            match &destination.kind {
                resolved::DestinationKind::Variable(variable) => {
                    let function = ctx
                        .resolved_ast
                        .functions
                        .get_mut(resolved_function_ref)
                        .unwrap();

                    function
                        .variables
                        .get(variable.key)
                        .expect("variable being assigned to exists")
                        .set_initialized();
                }
                resolved::DestinationKind::GlobalVariable(..) => (),
                resolved::DestinationKind::Member { .. } => (),
                resolved::DestinationKind::ArrayAccess { .. } => (),
                resolved::DestinationKind::Dereference { .. } => (),
            }

            let operator = assignment
                .operator
                .as_ref()
                .map(|ast_operator| {
                    resolve_basic_binary_operator(
                        ctx,
                        ast_operator,
                        &destination.resolved_type,
                        source,
                    )
                })
                .transpose()?;

            Ok(resolved::Stmt::new(
                resolved::StmtKind::Assignment(resolved::Assignment {
                    destination,
                    value: value.expr,
                    operator,
                }),
                source,
            ))
        }
    }
}
