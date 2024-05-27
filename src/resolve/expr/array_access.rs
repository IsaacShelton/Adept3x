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
        Some(PreferredType::of(
            &resolved::TypeKind::Integer {
                bits: IntegerBits::Bits64,
                sign: IntegerSign::Unsigned,
            }
            .at(source),
        )),
        Initialized::Require,
    )?;

    let item_type = match &subject.resolved_type.kind {
        resolved::TypeKind::Pointer(inner) => Ok((**inner).clone()),
        bad_type => Err(ResolveErrorKind::CannotAccessMemberOf {
            bad_type: bad_type.to_string(),
        }
        .at(source)),
    }?;

    if !item_type.kind.is_integer() {
        return Err(ResolveErrorKind::ExpectedIndexOfType {
            expected: "(any integer type)".to_string(),
            got: item_type.to_string(),
        }
        .at(source));
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
