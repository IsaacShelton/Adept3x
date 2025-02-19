use super::{resolve_expr, PreferredType, ResolveExprCtx, ResolveExprMode};
use crate::{
    asg::{self, TypedExpr},
    ast,
    resolve::{
        conform::to_default::conform_expr_to_default_or_error,
        destination::resolve_expr_to_destination,
        error::{ResolveError, ResolveErrorKind},
        Initialized,
    },
    source_files::Source,
};
use ast::{IntegerBits, IntegerSign};

pub fn resolve_array_access_expr(
    ctx: &mut ResolveExprCtx<'_, '_>,
    array_access: &ast::ArrayAccess,
    source: Source,
) -> Result<TypedExpr, ResolveError> {
    let c_integer_assumptions = ctx.c_integer_assumptions();

    let subject = conform_expr_to_default_or_error(
        resolve_expr(
            ctx,
            &array_access.subject,
            None,
            Initialized::Require,
            ResolveExprMode::RequireValue,
        )?,
        c_integer_assumptions,
    )?;

    let index = conform_expr_to_default_or_error(
        resolve_expr(
            ctx,
            &array_access.index,
            Some(PreferredType::of(
                &asg::TypeKind::Integer(IntegerBits::Bits64, IntegerSign::Unsigned).at(source),
            )),
            Initialized::Require,
            ResolveExprMode::RequireValue,
        )?,
        c_integer_assumptions,
    )?;

    let (item_type, subject) = match &subject.ty.kind {
        asg::TypeKind::Ptr(inner) => {
            let item_type = (**inner).clone();
            let array_destination = subject.expr.into();
            Ok((item_type, array_destination))
        }
        asg::TypeKind::FixedArray(fixed_array) => {
            let item_type = fixed_array.inner.clone();
            let array_destination = resolve_expr_to_destination(subject)?.into();
            Ok((item_type, array_destination))
        }
        bad_type => Err(ResolveErrorKind::CannotAccessMemberOf {
            bad_type: bad_type.to_string(),
        }
        .at(source)),
    }?;

    if !index.ty.kind.is_integer() {
        return Err(ResolveErrorKind::ExpectedIndexOfType {
            expected: "(any integer type)".to_string(),
            got: index.ty.to_string(),
        }
        .at(source));
    }

    Ok(TypedExpr::new(
        item_type.clone(),
        asg::Expr::new(
            asg::ExprKind::ArrayAccess(Box::new(asg::ArrayAccess {
                subject,
                index: index.expr,
                item_type,
            })),
            source,
        ),
    ))
}
