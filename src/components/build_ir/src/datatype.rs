use super::error::LowerError;
use crate::{ModBuilder, error::LowerErrorKind, structure::monomorphize_struct_with_params};
use data_units::ByteUnits;
use primitives::{CInteger, FloatSize, IntegerSign};
use std::borrow::{Borrow, Cow};
use target::{Target, TargetOsExt};
use target_layout::{TargetLayout, TypeLayout};

// Represents a resolved type that has had all its polymorphs resolved
pub struct ConcreteType<'a>(pub Cow<'a, asg::Type>);

pub fn lower_type(
    mod_builder: &ModBuilder,
    concrete_type: &ConcreteType,
) -> Result<ir::Type, LowerError> {
    use primitives::{IntegerBits as Bits, IntegerSign as Sign};

    let target = &mod_builder.target;
    let ty = &concrete_type.borrow().0;

    match &ty.kind {
        asg::TypeKind::Unresolved => panic!("got unresolved type during lower_type!"),
        asg::TypeKind::Polymorph(name) => Err(LowerError::other(
            format!("Cannot lower polymorph '${}' directly", name),
            ty.source,
        )),
        asg::TypeKind::Trait(name, _, _) => Err(LowerErrorKind::CannotUseTraitDirectly {
            name: name.to_string(),
        }
        .at(ty.source)),
        asg::TypeKind::Boolean => Ok(ir::Type::Bool),
        asg::TypeKind::Integer(bits, sign) => Ok(match (bits, sign) {
            (Bits::Bits8, Sign::Signed) => ir::Type::S8,
            (Bits::Bits8, Sign::Unsigned) => ir::Type::U8,
            (Bits::Bits16, Sign::Signed) => ir::Type::S16,
            (Bits::Bits16, Sign::Unsigned) => ir::Type::U16,
            (Bits::Bits32, Sign::Signed) => ir::Type::S32,
            (Bits::Bits32, Sign::Unsigned) => ir::Type::U32,
            (Bits::Bits64, Sign::Signed) => ir::Type::S64,
            (Bits::Bits64, Sign::Unsigned) => ir::Type::U64,
        }),
        asg::TypeKind::CInteger(integer, sign) => Ok(lower_c_integer(target, *integer, *sign)),
        asg::TypeKind::SizeInteger(sign) => {
            let layout = mod_builder.target.size_layout();
            assert_eq!(layout, TypeLayout::basic(ByteUnits::of(8)));

            Ok(match sign {
                Sign::Signed => ir::Type::S64,
                Sign::Unsigned => ir::Type::U64,
            })
        }
        asg::TypeKind::IntegerLiteral(value) => {
            Err(LowerErrorKind::CannotLowerUnspecializedIntegerLiteral {
                value: value.to_string(),
            }
            .at(ty.source))
        }
        asg::TypeKind::FloatLiteral(value) => {
            Err(LowerErrorKind::CannotLowerUnspecializedFloatLiteral {
                value: if let Some(value) = value {
                    value.to_string()
                } else {
                    "NaN".into()
                },
            }
            .at(ty.source))
        }
        asg::TypeKind::Floating(size) => Ok(match size {
            FloatSize::Bits32 => ir::Type::F32,
            FloatSize::Bits64 => ir::Type::F64,
        }),
        asg::TypeKind::Ptr(inner) => Ok(ir::Type::Ptr(Box::new(lower_type(
            mod_builder,
            &ConcreteType(Cow::Borrowed(inner)),
        )?))),
        asg::TypeKind::Void | asg::TypeKind::Never => Ok(ir::Type::Void),
        asg::TypeKind::Structure(_, struct_ref, parameters) => {
            // NOTE: We can assume that all parameters have been resolved to concrete types by this
            // point

            let mut values = Vec::with_capacity(parameters.len());
            for parameter in parameters {
                assert!(!parameter.kind.contains_polymorph());
                values.push(ConcreteType(Cow::Borrowed(parameter)));
            }

            monomorphize_struct_with_params(
                mod_builder,
                *struct_ref,
                values.as_slice(),
                concrete_type.0.source,
            )
            .map(ir::Type::Struct)
        }
        asg::TypeKind::AnonymousStruct() => {
            todo!("lower anonymous struct")
        }
        asg::TypeKind::AnonymousUnion() => {
            todo!("lower anonymous union")
        }
        asg::TypeKind::AnonymousEnum(anonymous_enum) => lower_type(
            mod_builder,
            &ConcreteType(Cow::Borrowed(&anonymous_enum.backing_type)),
        ),
        asg::TypeKind::FixedArray(fixed_array) => {
            let size = fixed_array.size;
            let inner = lower_type(
                mod_builder,
                &ConcreteType(Cow::Borrowed(&fixed_array.inner)),
            )?;

            Ok(ir::Type::FixedArray(Box::new(ir::FixedArray {
                length: size,
                inner,
            })))
        }
        asg::TypeKind::FuncPtr(_func_pointer) => Ok(ir::Type::FuncPtr),
        asg::TypeKind::Enum(_human_name, enum_ref) => {
            let enum_definition = &mod_builder.asg.enums[*enum_ref];

            lower_type(
                mod_builder,
                &ConcreteType(Cow::Borrowed(&enum_definition.ty)),
            )
        }
        asg::TypeKind::TypeAlias(_, type_alias_ref, type_args) => {
            if !type_args.is_empty() {
                todo!("lower_type for type alias with type args");
            }

            let type_alias = &mod_builder.asg.type_aliases[*type_alias_ref];

            lower_type(
                mod_builder,
                &ConcreteType(Cow::Borrowed(&type_alias.becomes)),
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
