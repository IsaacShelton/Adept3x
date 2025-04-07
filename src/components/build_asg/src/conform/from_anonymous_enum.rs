use super::{ConformMode, Objective, ObjectiveResult};
use asg::{AnonymousEnum, Cast, CastFrom, Expr, Type, TypeKind, TypedExpr};
use source_files::Source;

pub fn from_anonymous_enum<O: Objective>(
    expr: &Expr,
    _from_type: &Type,
    _mode: ConformMode,
    to_type: &Type,
    enumeration: &AnonymousEnum,
    source: Source,
) -> ObjectiveResult<O> {
    match &to_type.kind {
        TypeKind::Integer(..) | TypeKind::CInteger(..) => {
            if !enumeration.allow_implicit_integer_conversions {
                return O::fail();
            }

            O::success(|| {
                TypedExpr::new(
                    to_type.clone(),
                    asg::ExprKind::IntegerCast(Box::new(CastFrom {
                        cast: Cast {
                            target_type: to_type.clone(),
                            value: expr.clone(),
                        },
                        from_type: enumeration.backing_type.clone(),
                    }))
                    .at(source),
                )
            })
        }
        _ => O::fail(),
    }
}
