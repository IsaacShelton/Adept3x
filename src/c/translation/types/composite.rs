use super::get_name_and_type;
use crate::{
    ast::{AnonymousStruct, AstFile, Field, Privacy, Struct, TypeKind, TypeParams},
    c::parser::{
        error::ParseErrorKind, CTypedef, Composite, CompositeKind, DeclarationSpecifiers,
        MemberDeclaration, MemberDeclarator, ParseError,
    },
    diagnostics::Diagnostics,
    name::Name,
};
use indexmap::IndexMap;
use std::collections::HashMap;

pub fn make_composite(
    ast_file: &mut AstFile,
    typedefs: &HashMap<String, CTypedef>,
    composite: &Composite,
    diagnostics: &Diagnostics,
) -> Result<TypeKind, ParseError> {
    if !composite.attributes.is_empty() {
        return Err(
            ParseErrorKind::Misc("attributes not supported on composites yet").at(composite.source),
        );
    }

    let members = if let Some(members) = composite.members.as_ref() {
        members
    } else {
        let name = composite.name.as_ref().ok_or_else(|| {
            ParseErrorKind::Misc("incomplete struct must have name").at(composite.source)
        })?;

        return Ok(match &composite.kind {
            CompositeKind::Struct => {
                TypeKind::Named(Name::plain(format!("struct<{}>", name)), vec![])
            }
            CompositeKind::Union => {
                TypeKind::Named(Name::plain(format!("union<{}>", name)), vec![])
            }
        });
    };

    match &composite.kind {
        CompositeKind::Struct => {
            let mut fields = IndexMap::new();

            for member in members.iter() {
                match member {
                    MemberDeclaration::Member(member) => {
                        if !member.attributes.is_empty() {
                            todo!("attributes on members not supported yet");
                        }

                        for member_declarator in member.member_declarators.iter() {
                            match member_declarator {
                                MemberDeclarator::Declarator(declarator) => {
                                    let (name, ast_type, storage_class, function_specifier, _) =
                                        get_name_and_type(
                                            ast_file,
                                            typedefs,
                                            declarator,
                                            &DeclarationSpecifiers::from(
                                                &member.specifier_qualifiers,
                                            ),
                                            false,
                                            diagnostics,
                                        )?;

                                    if storage_class.is_some() {
                                        return Err(ParseErrorKind::Misc(
                                            "Storage classes not supported here",
                                        )
                                        .at(declarator.source));
                                    }

                                    if function_specifier.is_some() {
                                        return Err(ParseErrorKind::Misc(
                                            "Function specifiers cannot be used here",
                                        )
                                        .at(declarator.source));
                                    }

                                    fields.insert(
                                        name.clone(),
                                        Field {
                                            ast_type,
                                            privacy: Privacy::Public,
                                            source: declarator.source,
                                        },
                                    );
                                }
                                MemberDeclarator::BitField(_, _) => {
                                    todo!("bitfield members not supported yet")
                                }
                            }
                        }
                    }
                    MemberDeclaration::StaticAssert(_) => {
                        todo!("static assert as member in struct")
                    }
                }
            }

            let is_packed = false;

            if let Some(name) = &composite.name {
                let name = format!("struct<{}>", name);

                ast_file.structs.push(Struct {
                    name: name.clone(),
                    params: TypeParams::default(),
                    fields,
                    is_packed,
                    source: composite.source,
                    privacy: Privacy::Private,
                });

                Ok(TypeKind::Named(Name::plain(name), vec![]))
            } else {
                let anonymous_struct = AnonymousStruct { fields, is_packed };

                Ok(TypeKind::AnonymousStruct(anonymous_struct))
            }
        }
        CompositeKind::Union => {
            todo!("union composites")
        }
    }
}
