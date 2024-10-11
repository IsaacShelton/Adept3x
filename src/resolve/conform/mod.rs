mod from_c_integer;
mod from_float;
mod from_float_literal;
mod from_integer;
mod from_integer_literal;
mod from_pointer;
mod mode;
pub mod to_default;

use self::{
    from_c_integer::from_c_integer, from_float::from_float, from_float_literal::from_float_literal,
    from_integer::from_integer, from_integer_literal::from_integer_literal,
    from_pointer::from_pointer,
};
use super::error::{ResolveError, ResolveErrorKind};
use crate::{
    ast::ConformBehavior,
    resolved::{Type, TypeKind, TypedExpr},
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

pub fn conform_expr<O: Objective>(
    expr: &TypedExpr,
    to_type: &Type,
    mode: ConformMode,
    behavior: ConformBehavior,
    conform_source: Source,
) -> ObjectiveResult<O> {
    if expr.resolved_type == *to_type {
        return O::success(|| TypedExpr {
            resolved_type: to_type.clone(),
            expr: expr.expr.clone(),
            is_initialized: expr.is_initialized,
        });
    }

    match &expr.resolved_type.kind {
        TypeKind::IntegerLiteral(from) => from_integer_literal::<O>(
            from,
            behavior.c_integer_assumptions(),
            expr.expr.source,
            to_type,
        ),
        TypeKind::Integer(from_bits, from_sign) => {
            from_integer::<O>(expr, mode, behavior, *from_bits, *from_sign, to_type)
        }
        TypeKind::FloatLiteral(from) => from_float_literal::<O>(*from, to_type, conform_source),
        TypeKind::Floating(from_size) => from_float::<O>(expr, mode, *from_size, to_type),
        TypeKind::Pointer(from_inner) => from_pointer::<O>(expr, mode, from_inner, to_type),
        TypeKind::CInteger(from_size, from_sign) => {
            from_c_integer::<O>(expr, mode, *from_size, *from_sign, to_type, conform_source)
        }
        _ => O::fail(),
    }
}

pub fn conform_expr_or_error(
    expr: &TypedExpr,
    target_type: &Type,
    mode: ConformMode,
    behavior: ConformBehavior,
    conform_source: Source,
) -> Result<TypedExpr, ResolveError> {
    conform_expr::<Perform>(expr, target_type, mode, behavior, conform_source).or_else(|_| {
        Err(ResolveErrorKind::TypeMismatch {
            left: expr.resolved_type.to_string(),
            right: target_type.to_string(),
        }
        .at(expr.expr.source))
    })
}
