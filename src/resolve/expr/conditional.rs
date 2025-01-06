use super::{resolve_expr, PreferredType, ResolveExprCtx, ResolveExprMode};
use crate::{
    asg::{self, Branch, TypedExpr},
    ast,
    resolve::{
        conform::{conform_expr_or_error, ConformMode},
        error::{ResolveError, ResolveErrorKind},
        resolve_stmts,
        unify_types::unify_types,
        Initialized,
    },
    source_files::Source,
};
use itertools::Itertools;

pub fn resolve_conditional_expr(
    ctx: &mut ResolveExprCtx<'_, '_>,
    conditional: &ast::Conditional,
    preferred_type: Option<PreferredType>,
    mode: ResolveExprMode,
    source: Source,
) -> Result<TypedExpr, ResolveError> {
    let ast::Conditional {
        conditions,
        otherwise,
    } = conditional;

    let mut branches_without_else = Vec::with_capacity(conditions.len());

    for (expr, block) in conditions.iter() {
        ctx.variable_haystack.begin_scope();
        let condition = resolve_expr(
            ctx,
            expr,
            preferred_type,
            Initialized::Require,
            ResolveExprMode::RequireValue,
        )?;

        let stmts = resolve_stmts(ctx, &block.stmts, mode)?;

        let condition = conform_expr_or_error(
            ctx,
            &condition,
            &asg::TypeKind::Boolean.at(source),
            ConformMode::Normal,
            ctx.adept_conform_behavior(),
            source,
        )?;

        branches_without_else.push(Branch {
            condition,
            block: asg::Block::new(stmts),
        });

        ctx.variable_haystack.end_scope();
    }

    let mut otherwise = otherwise
        .as_ref()
        .map(|otherwise| {
            ctx.variable_haystack.begin_scope();
            let maybe_block = resolve_stmts(ctx, &otherwise.stmts, mode).map(asg::Block::new);
            ctx.variable_haystack.end_scope();
            maybe_block
        })
        .transpose()?;

    let block_results = branches_without_else
        .iter()
        .map(|branch| &branch.block)
        .chain(otherwise.iter())
        .map(|block| block.get_result_type(source))
        .collect_vec();

    let result_type: Option<asg::Type> = if mode == ResolveExprMode::NeglectValue {
        None
    } else if block_results
        .iter()
        .any(|result| result.kind == asg::TypeKind::Void)
    {
        Some(
            block_results
                .iter()
                .all_equal()
                .then_some(asg::TypeKind::Void.at(source))
                .ok_or_else(|| {
                    ResolveErrorKind::MismatchingYieldedTypes {
                        got: block_results
                            .iter()
                            .map(|ty| ty.kind.to_string())
                            .collect_vec(),
                    }
                    .at(source)
                })?,
        )
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
                    asg::StmtKind::Expr(expr) => expr,
                    asg::StmtKind::Return(..)
                    | asg::StmtKind::Declaration(..)
                    | asg::StmtKind::Assignment(..) => unreachable!(),
                }
            })
            .collect_vec();

        unify_types(
            ctx,
            preferred_type.map(|preferred_type| preferred_type.view(ctx.asg)),
            &mut last_exprs[..],
            ctx.adept_conform_behavior(),
            source,
        )
        .map(Some)
        .ok_or_else(|| {
            ResolveErrorKind::MismatchingYieldedTypes {
                got: block_results
                    .iter()
                    .map(|ty| ty.kind.to_string())
                    .collect_vec(),
            }
            .at(source)
        })?
    };

    let expr = asg::Expr::new(
        asg::ExprKind::Conditional(Box::new(asg::Conditional {
            result_type: result_type.clone(),
            branches: branches_without_else,
            otherwise,
        })),
        source,
    );

    Ok(TypedExpr::new(
        result_type.unwrap_or_else(|| asg::TypeKind::Void.at(source)),
        expr,
    ))
}
