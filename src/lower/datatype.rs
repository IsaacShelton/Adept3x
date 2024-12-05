use super::error::LowerError;
use crate::{
    ast::{CInteger, FloatSize},
    ir::{self, IntegerSign},
    lower::error::LowerErrorKind,
    resolved,
    target::{Target, TargetOsExt},
};
use std::borrow::{Borrow, Cow};

// Represents a resolved type that has had all its polymorphs resolved
pub struct ConcreteType<'a>(pub Cow<'a, resolved::Type>);

pub fn lower_type(
    target: &Target,
    concrete_type: &ConcreteType,
    resolved_ast: &resolved::Ast,
) -> Result<ir::Type, LowerError> {
    use resolved::{IntegerBits as Bits, IntegerSign as Sign};

    let resolved_type = &concrete_type.borrow().0;

    match &resolved_type.kind {
        resolved::TypeKind::Unresolved => panic!("got unresolved type during lower_type!"),
        resolved::TypeKind::Polymorph(_, _) => todo!("cannot directly lower polymorph"),
        resolved::TypeKind::Boolean => Ok(ir::Type::Boolean),
        resolved::TypeKind::Integer(bits, sign) => Ok(match (bits, sign) {
            (Bits::Bits8, Sign::Signed) => ir::Type::S8,
            (Bits::Bits8, Sign::Unsigned) => ir::Type::U8,
            (Bits::Bits16, Sign::Signed) => ir::Type::S16,
            (Bits::Bits16, Sign::Unsigned) => ir::Type::U16,
            (Bits::Bits32, Sign::Signed) => ir::Type::S32,
            (Bits::Bits32, Sign::Unsigned) => ir::Type::U32,
            (Bits::Bits64, Sign::Signed) => ir::Type::S64,
            (Bits::Bits64, Sign::Unsigned) => ir::Type::U64,
        }),
        resolved::TypeKind::CInteger(integer, sign) => Ok(lower_c_integer(target, *integer, *sign)),
        resolved::TypeKind::IntegerLiteral(value) => {
            Err(LowerErrorKind::CannotLowerUnspecializedIntegerLiteral {
                value: value.to_string(),
            }
            .at(resolved_type.source))
        }
        resolved::TypeKind::FloatLiteral(value) => {
            Err(LowerErrorKind::CannotLowerUnspecializedFloatLiteral {
                value: if let Some(value) = value {
                    value.to_string()
                } else {
                    "NaN".into()
                },
            }
            .at(resolved_type.source))
        }
        resolved::TypeKind::Floating(size) => Ok(match size {
            FloatSize::Bits32 => ir::Type::F32,
            FloatSize::Bits64 => ir::Type::F64,
        }),
        resolved::TypeKind::Pointer(inner) => Ok(ir::Type::Pointer(Box::new(lower_type(
            target,
            &ConcreteType(Cow::Borrowed(inner)),
            resolved_ast,
        )?))),
        resolved::TypeKind::Void => Ok(ir::Type::Void),
        resolved::TypeKind::Structure(_, structure_ref, parameters) => {
            todo!("instantiate structure");
            // Ok(ir::Type::Structure(*structure_ref))
        }
        resolved::TypeKind::AnonymousStruct() => {
            todo!("lower anonymous struct")
        }
        resolved::TypeKind::AnonymousUnion() => {
            todo!("lower anonymous union")
        }
        resolved::TypeKind::AnonymousEnum() => {
            todo!("lower anonymous enum")
        }
        resolved::TypeKind::FixedArray(fixed_array) => {
            let size = fixed_array.size;
            let inner = lower_type(
                target,
                &ConcreteType(Cow::Borrowed(&fixed_array.inner)),
                resolved_ast,
            )?;

            Ok(ir::Type::FixedArray(Box::new(ir::FixedArray {
                length: size,
                inner,
            })))
        }
        resolved::TypeKind::FunctionPointer(_function_pointer) => Ok(ir::Type::FunctionPointer),
        resolved::TypeKind::Enum(_human_name, enum_ref) => {
            let enum_definition = resolved_ast
                .enums
                .get(*enum_ref)
                .expect("referenced enum to exist");

            lower_type(
                target,
                &ConcreteType(Cow::Borrowed(&enum_definition.resolved_type)),
                resolved_ast,
            )
        }
        resolved::TypeKind::TypeAlias(_, type_alias_ref) => {
            let resolved_type = resolved_ast
                .type_aliases
                .get(*type_alias_ref)
                .expect("referenced type alias to exist");

            lower_type(
                target,
                &ConcreteType(Cow::Borrowed(resolved_type)),
                resolved_ast,
            )
        }
    }
}

pub fn lower_c_integer(target: &Target, integer: CInteger, sign: Option<IntegerSign>) -> ir::Type {
    let sign = sign.unwrap_or_else(|| target.default_c_integer_sign(integer));

    match (integer, sign) {
        (CInteger::Char, IntegerSign::Signed) => ir::Type::S8,
        (CInteger::Char, IntegerSign::Unsigned) => ir::Type::U8,
        (CInteger::Short, IntegerSign::Signed) => ir::Type::S16,
        (CInteger::Short, IntegerSign::Unsigned) => ir::Type::U16,
        (CInteger::Int, IntegerSign::Signed) => ir::Type::S32,
        (CInteger::Int, IntegerSign::Unsigned) => ir::Type::U32,
        (CInteger::Long, IntegerSign::Signed) => {
            if target.os().is_windows() {
                ir::Type::S32
            } else {
                ir::Type::S64
            }
        }
        (CInteger::Long, IntegerSign::Unsigned) => {
            if target.os().is_windows() {
                ir::Type::U32
            } else {
                ir::Type::U64
            }
        }
        (CInteger::LongLong, IntegerSign::Signed) => ir::Type::S64,
        (CInteger::LongLong, IntegerSign::Unsigned) => ir::Type::U64,
    }
}
