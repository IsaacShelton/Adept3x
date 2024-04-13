use super::{resolve_expr, PreferredType, ResolveExprCtx};
use crate::{
    ast::{self, Source},
    resolve::{
        error::{ResolveError, ResolveErrorKind},
        Initialized,
    },
    resolved::{self, TypedExpr},
};
use ast::{IntegerBits, IntegerSign};

pub fn resolve_array_access_expr(
    ctx: &mut ResolveExprCtx<'_, '_>,
    array_access: &ast::ArrayAccess,
    source: Source,
) -> Result<TypedExpr, ResolveError> {
    let subject = resolve_expr(
        ctx,
        &array_access.subject,
        None,
        crate::resolve::Initialized::Require,
    )?;

    let index = resolve_expr(
        ctx,
        &array_access.index,
        Some(PreferredType::of(&resolved::Type::Integer {
            bits: IntegerBits::Bits64,
            sign: IntegerSign::Unsigned,
        })),
        Initialized::Require,
    )?;

    let item_type = match subject.resolved_type {
        resolved::Type::Pointer(inner) => Ok(*inner),
        bad_type => Err(ResolveError::new(
            ctx.resolved_ast.source_file_cache,
            source,
            ResolveErrorKind::CannotAccessMemberOf {
                bad_type: bad_type.to_string(),
            },
        )),
    }?;

    if !item_type.is_integer() {
        return Err(ResolveError::new(
            ctx.resolved_ast.source_file_cache,
            source,
            ResolveErrorKind::ExpectedIndexOfType {
                expected: "(any integer type)".to_string(),
                got: item_type.to_string(),
            },
        ));
    }

    Ok(TypedExpr::new(
        item_type.clone(),
        resolved::Expr::new(
            resolved::ExprKind::ArrayAccess(Box::new(resolved::ArrayAccess {
                subject: subject.expr,
                index: index.expr,
                item_type,
            })),
            source,
        ),
    ))
}
