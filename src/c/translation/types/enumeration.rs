use crate::{
    ast::{self, AnonymousEnum, AstFile, EnumMember, TypeKind},
    c::{
        parser::{error::ParseErrorKind, Enumeration, ParseError},
        translation::eval::evaluate_to_const_integer,
    },
    index_map_ext::IndexMapExt,
};
use indexmap::IndexMap;
use num_bigint::BigInt;
use num_traits::Zero;

pub fn make_anonymous_enum(
    ast_file: &mut AstFile,
    enumeration: &Enumeration,
) -> Result<TypeKind, ParseError> {
    match enumeration {
        Enumeration::Definition(definition) => {
            if !definition.attributes.is_empty() {
                todo!("enum attributes not supported yet")
            }

            let mut members = IndexMap::with_capacity(definition.body.len());
            let mut next_value = BigInt::zero();

            for enumerator in definition.body.iter() {
                if !enumerator.attributes.is_empty() {
                    todo!("attributes not supported on enum members yet");
                }

                let value = if let Some(value) = &enumerator.value {
                    evaluate_to_const_integer(&value.value)?
                } else {
                    let value = next_value.clone();
                    next_value += 1;
                    value
                };

                let enum_member = EnumMember {
                    value,
                    explicit_value: enumerator.value.is_some(),
                };

                if members
                    .insert(enumerator.name.clone(), enum_member)
                    .is_some()
                {
                    return Err(ParseErrorKind::DuplicateEnumMember(enumerator.name.clone())
                        .at(enumerator.source));
                }

                // TODO: Add way to use enums that don't have a definition name
                // Should they just be normal defines? Or anonymous enum values? (which don't exist yet)
                if let Some(definition_name) = &definition.name {
                    let aka_value =
                        ast::ExprKind::EnumMemberLiteral(Box::new(ast::EnumMemberLiteral {
                            enum_name: format!("enum<{}>", definition_name),
                            variant_name: enumerator.name.clone(),
                            source: enumerator.source,
                        }))
                        .at(enumerator.source);

                    ast_file.helper_exprs.try_insert(
                        enumerator.name.clone(),
                        ast::HelperExpr {
                            value: aka_value,
                            source: enumerator.source,
                            is_file_local_only: false,
                        },
                        |name| {
                            ParseErrorKind::EnumMemberNameConflictsWithExistingSymbol { name }
                                .at(enumerator.source)
                        },
                    )?;
                }
            }

            let backing_type = if definition.enum_type_specifier.is_some() {
                todo!("anonymous enum type specifiers not supported yet");
            } else {
                None
            };

            Ok(TypeKind::AnonymousEnum(AnonymousEnum {
                members,
                backing_type,
            }))
        }
        Enumeration::Named(named) => {
            if named.enum_type_specifier.is_some() {
                todo!("support enum type specifiers")
            }

            Ok(TypeKind::Named(format!("enum<{}>", named.name)))
        }
    }
}
