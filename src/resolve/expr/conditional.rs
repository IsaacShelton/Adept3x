use super::{resolve_expr, PreferredType, ResolveExprCtx};
use crate::{
    ast::{self, Source},
    resolve::{
        conform_expr_or_error,
        error::{ResolveError, ResolveErrorKind},
        resolve_stmts,
        unify_types::unify_types,
        ConformMode, Initialized,
    },
    resolved::{self, Branch, TypedExpr},
};
use itertools::Itertools;

pub fn resolve_conditional_expr(
    ctx: &mut ResolveExprCtx<'_, '_>,
    conditional: &ast::Conditional,
    preferred_type: Option<PreferredType>,
    source: Source,
) -> Result<TypedExpr, ResolveError> {
    let ast::Conditional {
        conditions,
        otherwise,
    } = conditional;

    let mut branches_without_else = Vec::with_capacity(conditions.len());

    for (expr, block) in conditions.iter() {
        ctx.variable_search_ctx.begin_scope();
        let condition = resolve_expr(ctx, expr, preferred_type, Initialized::Require)?;

        let stmts = resolve_stmts(ctx, &block.stmts)?;

        let condition = conform_expr_or_error(
            &condition,
            &resolved::TypeKind::Boolean.at(source),
            ConformMode::Normal,
            source,
        )?;

        branches_without_else.push(Branch {
            condition,
            block: resolved::Block::new(stmts),
        });

        ctx.variable_search_ctx.end_scope();
    }

    let mut otherwise = otherwise
        .as_ref()
        .map(|otherwise| {
            ctx.variable_search_ctx.begin_scope();
            let maybe_block =
                resolve_stmts(ctx, &otherwise.stmts).map(|stmts| resolved::Block::new(stmts));
            ctx.variable_search_ctx.end_scope();
            maybe_block
        })
        .transpose()?;

    let block_results = branches_without_else
        .iter()
        .map(|branch| &branch.block)
        .chain(otherwise.iter())
        .map(|block| block.get_result_type(source))
        .collect_vec();

    let result_type = if block_results
        .iter()
        .any(|result| result.kind == resolved::TypeKind::Void)
    {
        block_results
            .iter()
            .all_equal()
            .then_some(resolved::TypeKind::Void.at(source))
            .ok_or_else(|| {
                ResolveErrorKind::MismatchingYieldedTypes {
                    got: block_results
                        .iter()
                        .map(|resolved_type| resolved_type.kind.to_string())
                        .collect_vec(),
                }
                .at(source)
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
                    resolved::StmtKind::Return(..)
                    | resolved::StmtKind::Declaration(..)
                    | resolved::StmtKind::Assignment(..) => unreachable!(),
                }
            })
            .collect_vec();

        unify_types(
            preferred_type.map(|preferred_type| preferred_type.view(ctx.resolved_ast)),
            &mut last_exprs[..],
            source,
        )
        .ok_or_else(|| {
            ResolveErrorKind::MismatchingYieldedTypes {
                got: block_results
                    .iter()
                    .map(|resolved_type| resolved_type.kind.to_string())
                    .collect_vec(),
            }
            .at(source)
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
