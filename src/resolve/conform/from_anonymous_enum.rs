use super::{ConformMode, Objective, ObjectiveResult};
use crate::{
    asg::{AnonymousEnum, Expr, Type, TypeKind},
    source_files::Source,
};

pub fn from_anonymous_enum<O: Objective>(
    _expr: &Expr,
    _from_type: &Type,
    _mode: ConformMode,
    to_type: &Type,
    enumeration: &AnonymousEnum,
    _source: Source,
) -> ObjectiveResult<O> {
    match &to_type.kind {
        TypeKind::Integer(_to_bits, _to_sign) => {
            if !enumeration.allow_implicit_integer_conversions {
                return O::fail();
            }

            todo!("convert from anonymous enum to fixed integer")
        }
        TypeKind::CInteger(_to_c_integer, _to_sign) => {
            if !enumeration.allow_implicit_integer_conversions {
                return O::fail();
            }

            todo!("convert from anonymous enum to flexible integer")
        }
        _ => O::fail(),
    }
}
