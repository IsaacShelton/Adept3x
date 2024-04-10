use super::{resolve_expr, ResolveExprCtx};
use crate::{
    ast::{self, Source},
    resolve::{
        conform_expr_or_error,
        error::{ResolveError, ResolveErrorKind},
        resolve_stmts, unify_types, Initialized,
    },
    resolved::{self, Branch, TypedExpr},
};
use itertools::Itertools;

pub fn resolve_conditional_expr(
    ctx: &mut ResolveExprCtx<'_, '_>,
    conditional: &ast::Conditional,
    source: Source,
) -> Result<TypedExpr, ResolveError> {
    let ast::Conditional {
        conditions,
        otherwise,
    } = conditional;

    let mut otherwise = otherwise
        .as_ref()
        .map(|otherwise| {
            resolve_stmts(ctx, &otherwise.stmts).map(|stmts| resolved::Block::new(stmts))
        })
        .transpose()?;

    let mut branches_without_else = Vec::with_capacity(conditions.len());

    for (expr, block) in conditions.iter() {
        let condition = resolve_expr(ctx, expr, Initialized::Require)?;
        let stmts = resolve_stmts(ctx, &block.stmts)?;

        let condition = conform_expr_or_error(
            ctx.resolved_ast.source_file_cache,
            &condition,
            &resolved::Type::Boolean,
        )?;

        branches_without_else.push(Branch {
            condition,
            block: resolved::Block::new(stmts),
        });
    }

    let block_results = branches_without_else
        .iter()
        .map(|branch| &branch.block)
        .chain(otherwise.iter())
        .map(|block| block.get_result_type())
        .collect_vec();

    let result_type = if block_results
        .iter()
        .any(|result| result == &resolved::Type::Void)
    {
        block_results
            .iter()
            .all_equal()
            .then_some(resolved::Type::Void)
            .ok_or_else(|| {
                ResolveError::new(
                    ctx.resolved_ast.source_file_cache,
                    source,
                    ResolveErrorKind::MismatchingYieldedTypes {
                        got: block_results
                            .iter()
                            .map(|resolved_type| resolved_type.to_string())
                            .collect_vec(),
                    },
                )
            })
    } else {
        let mut last_exprs = branches_without_else
            .chunks_exact_mut(1)
            .map(|branch| &mut branch[0].block)
            .chain(otherwise.iter_mut())
            .map(|block| {
                match &mut block
                    .stmts
                    .last_mut()
                    .expect("last statement to exist")
                    .kind
                {
                    resolved::StmtKind::Expr(expr) => expr,
                    resolved::StmtKind::Return(_)
                    | resolved::StmtKind::Declaration(_)
                    | resolved::StmtKind::Assignment(_) => unreachable!(),
                }
            })
            .collect_vec();

        unify_types(&mut last_exprs[..]).ok_or_else(|| {
            ResolveError::new(
                ctx.resolved_ast.source_file_cache,
                source,
                ResolveErrorKind::MismatchingYieldedTypes {
                    got: block_results
                        .iter()
                        .map(|resolved_type| resolved_type.to_string())
                        .collect_vec(),
                },
            )
        })
    }?;

    let expr = resolved::Expr::new(
        resolved::ExprKind::Conditional(resolved::Conditional {
            result_type: result_type.clone(),
            branches: branches_without_else,
            otherwise,
        }),
        source,
    );

    Ok(TypedExpr::new(result_type, expr))
}
