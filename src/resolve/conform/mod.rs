mod from_anonymous_enum;
mod from_c_integer;
mod from_float;
mod from_float_literal;
mod from_integer;
mod from_integer_literal;
mod from_pointer;
mod mode;
pub mod to_default;

use self::{
    from_anonymous_enum::from_anonymous_enum, from_c_integer::from_c_integer,
    from_float::from_float, from_float_literal::from_float_literal, from_integer::from_integer,
    from_integer_literal::from_integer_literal, from_pointer::from_pointer,
};
use super::{
    error::{ResolveError, ResolveErrorKind},
    expr::ResolveExprCtx,
};
use crate::{
    asg::{Type, TypeKind, TypedExpr},
    ast::ConformBehavior,
    source_files::Source,
};
pub use mode::ConformMode;

type ObjectiveResult<O> = Result<<O as Objective>::Success, <O as Objective>::Failure>;
pub trait Objective {
    type Success;
    type Failure;
    fn success(x: impl FnOnce() -> TypedExpr) -> ObjectiveResult<Self>;
    fn fail() -> ObjectiveResult<Self>;
}
pub struct Perform;
pub struct Validate;
impl Objective for Perform {
    type Success = TypedExpr;
    type Failure = ();

    fn success(f: impl FnOnce() -> TypedExpr) -> ObjectiveResult<Self> {
        Ok(f())
    }

    fn fail() -> ObjectiveResult<Self> {
        Err(())
    }
}
impl Objective for Validate {
    type Failure = ();
    type Success = ();

    fn success(_: impl FnOnce() -> TypedExpr) -> ObjectiveResult<Self> {
        Ok(())
    }

    fn fail() -> ObjectiveResult<Self> {
        Err(())
    }
}

pub fn warn_type_alias_depth_exceeded(ty: &Type) {
    // TODO: WARNING: When this happens, it might not be obvious why there
    // wasn't a match. This should probably cause an error message
    // TODO: Make this more transparent by adding a good error message
    eprintln!(
        "warning: ignoring type '{}' since it exceeds maximum type alias recursion depth",
        ty
    );
}

pub fn conform_expr<O: Objective>(
    ctx: &ResolveExprCtx,
    expr: &TypedExpr,
    to_type: &Type,
    mode: ConformMode,
    behavior: ConformBehavior,
    conform_source: Source,
) -> ObjectiveResult<O> {
    let Ok(from_type) = ctx.asg.unalias(&expr.ty) else {
        warn_type_alias_depth_exceeded(&expr.ty);
        return O::fail();
    };

    let Ok(to_type) = ctx.asg.unalias(to_type) else {
        warn_type_alias_depth_exceeded(to_type);
        return O::fail();
    };

    if *from_type == *to_type {
        return O::success(|| TypedExpr {
            ty: to_type.clone(),
            expr: expr.expr.clone(),
            is_initialized: expr.is_initialized,
        });
    }

    match &from_type.kind {
        TypeKind::IntegerLiteral(from) => from_integer_literal::<O>(
            from,
            behavior.c_integer_assumptions(),
            expr.expr.source,
            to_type,
        ),
        TypeKind::Integer(from_bits, from_sign) => from_integer::<O>(
            &expr.expr, from_type, mode, behavior, *from_bits, *from_sign, to_type,
        ),
        TypeKind::FloatLiteral(from) => from_float_literal::<O>(*from, to_type, conform_source),
        TypeKind::Floating(from_size) => from_float::<O>(&expr.expr, mode, *from_size, to_type),
        TypeKind::Ptr(from_inner) => from_pointer::<O>(ctx, &expr.expr, mode, from_inner, to_type),
        TypeKind::CInteger(from_size, from_sign) => from_c_integer::<O>(
            &expr.expr,
            from_type,
            mode,
            *from_size,
            *from_sign,
            to_type,
            conform_source,
        ),
        TypeKind::AnonymousEnum(enumeration) => from_anonymous_enum::<O>(
            &expr.expr,
            from_type,
            mode,
            to_type,
            enumeration.as_ref(),
            conform_source,
        ),
        _ => O::fail(),
    }
}

pub fn conform_expr_or_error(
    ctx: &ResolveExprCtx,
    expr: &TypedExpr,
    target_type: &Type,
    mode: ConformMode,
    behavior: ConformBehavior,
    conform_source: Source,
) -> Result<TypedExpr, ResolveError> {
    conform_expr::<Perform>(ctx, expr, target_type, mode, behavior, conform_source).or_else(|_| {
        Err(ResolveErrorKind::TypeMismatch {
            left: expr.ty.to_string(),
            right: target_type.to_string(),
        }
        .at(expr.expr.source))
    })
}
