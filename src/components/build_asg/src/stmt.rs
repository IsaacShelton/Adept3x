use super::{
    Initialized,
    conform::{ConformMode, Perform, conform_expr},
    destination::resolve_expr_to_destination,
    error::{ResolveError, ResolveErrorKind},
    expr::{
        PreferredType, ResolveExprCtx, ResolveExprMode, resolve_basic_binary_operator, resolve_expr,
    },
    type_ctx::ResolveTypeOptions,
};
use asg;
use ast;
use std::borrow::Cow;

pub fn resolve_stmts(
    ctx: &mut ResolveExprCtx,
    stmts: &[ast::Stmt],
    mode: ResolveExprMode,
) -> Result<Vec<asg::Stmt>, ResolveError> {
    let mut resolved_stmts = Vec::with_capacity(stmts.len());

    for stmt in stmts.iter() {
        resolved_stmts.push(resolve_stmt(ctx, stmt, mode)?);
    }

    Ok(resolved_stmts)
}

pub fn resolve_stmt(
    ctx: &mut ResolveExprCtx,
    ast_stmt: &ast::Stmt,
    mode: ResolveExprMode,
) -> Result<asg::Stmt, ResolveError> {
    let source = ast_stmt.source;

    match &ast_stmt.kind {
        ast::StmtKind::Return(value) => {
            let Some(func_ref) = ctx.func_ref else {
                return Err(ResolveErrorKind::CannotReturnOutsideFunction.at(ast_stmt.source));
            };

            let return_value = if let Some(value) = value {
                let result = resolve_expr(
                    ctx,
                    value,
                    Some(PreferredType::ReturnType(func_ref)),
                    Initialized::Require,
                    ResolveExprMode::RequireValue,
                )?;

                let return_type = Cow::Borrowed(&ctx.asg.funcs[func_ref].return_type);

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
                        returning: result.ty.to_string(),
                        expected: return_type.to_string(),
                    }
                    .at(source));
                }
            } else {
                let function = &ctx.asg.funcs[func_ref];

                if function.return_type.kind != asg::TypeKind::Void {
                    return Err(ResolveErrorKind::CannotReturnVoid {
                        expected: function.return_type.to_string(),
                    }
                    .at(source));
                }

                None
            };

            Ok(asg::Stmt::new(asg::StmtKind::Return(return_value), source))
        }
        ast::StmtKind::Expr(value) => Ok(asg::Stmt::new(
            asg::StmtKind::Expr(resolve_expr(ctx, value, None, Initialized::Require, mode)?),
            source,
        )),
        ast::StmtKind::Declaration(declaration) => {
            let ty = ctx
                .type_ctx()
                .resolve(&declaration.ast_type, ResolveTypeOptions::Unalias)?;

            let value = declaration
                .initial_value
                .as_ref()
                .map(|value| {
                    resolve_expr(
                        ctx,
                        value,
                        Some(PreferredType::of(&ty)),
                        Initialized::Require,
                        ResolveExprMode::RequireValue,
                    )
                })
                .transpose()?
                .as_ref()
                .map(|value| {
                    conform_expr::<Perform>(
                        ctx,
                        value,
                        &ty,
                        ConformMode::Normal,
                        ctx.adept_conform_behavior(),
                        source,
                    )
                    .map(|value| value.expr)
                    .map_err(|_| {
                        ResolveErrorKind::CannotAssignValueOfType {
                            from: value.ty.to_string(),
                            to: ty.to_string(),
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

            let Some(func_ref) = ctx.func_ref else {
                return Err(
                    ResolveErrorKind::CannotDeclareVariableOutsideFunction.at(ast_stmt.source)
                );
            };

            let key = ctx.asg.funcs[func_ref].vars.add_variable(ty.clone());

            ctx.variable_haystack
                .put(&declaration.name, ty.clone(), key);

            Ok(asg::Stmt::new(
                asg::StmtKind::Declaration(asg::Declaration { key, value }),
                source,
            ))
        }
        ast::StmtKind::Assignment(assignment) => {
            let destination_expr = resolve_expr(
                ctx,
                &assignment.destination,
                None,
                Initialized::AllowUninitialized,
                ResolveExprMode::RequireValue,
            )?;

            let value = resolve_expr(
                ctx,
                &assignment.value,
                Some(PreferredType::of(&destination_expr.ty)),
                Initialized::Require,
                ResolveExprMode::RequireValue,
            )?;

            let value = conform_expr::<Perform>(
                ctx,
                &value,
                &destination_expr.ty,
                ConformMode::Normal,
                ctx.adept_conform_behavior(),
                source,
            )
            .map_err(|_| {
                ResolveErrorKind::CannotAssignValueOfType {
                    from: value.ty.to_string(),
                    to: destination_expr.ty.to_string(),
                }
                .at(source)
            })?;

            let destination = resolve_expr_to_destination(destination_expr)?;

            if ctx.func_ref.is_none() {
                return Err(
                    ResolveErrorKind::CannotAssignVariableOutsideFunction.at(ast_stmt.source)
                );
            };

            let operator = assignment
                .operator
                .as_ref()
                .map(|ast_operator| {
                    resolve_basic_binary_operator(ctx, ast_operator, &destination.ty, source)
                })
                .transpose()?;

            Ok(asg::Stmt::new(
                asg::StmtKind::Assignment(asg::Assignment {
                    destination,
                    value: value.expr,
                    operator,
                }),
                source,
            ))
        }
        ast::StmtKind::Label(_) => unimplemented!("Label for old resolution system"),
        ast::StmtKind::Goto(_) => unimplemented!("Goto for old resolution system"),
        ast::StmtKind::ExitInterpreter(_) => {
            unimplemented!("ExitInterpreter for old resolution system")
        }
    }
}
