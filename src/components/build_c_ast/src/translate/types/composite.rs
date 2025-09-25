use super::get_name_and_type;
use crate::{
    parse::{ParseError, error::ParseErrorKind},
    translate::TranslateCtx,
};
use attributes::Privacy;
use c_ast::{Composite, CompositeKind, DeclarationSpecifiers, MemberDeclaration, MemberDeclarator};
use indexmap::IndexMap;

pub fn make_composite(
    ctx: &mut TranslateCtx,
    composite: &Composite,
) -> Result<ast::TypeKind, ParseError> {
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
            CompositeKind::Struct => ast::TypeKind::Named(
                ast::NamePath::new_plain(format!("struct<{}>", name)),
                vec![],
            ),
            CompositeKind::Union => {
                ast::TypeKind::Named(ast::NamePath::new_plain(format!("union<{}>", name)), vec![])
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
                                    let member_info = get_name_and_type(
                                        ctx,
                                        declarator,
                                        &DeclarationSpecifiers::from(&member.specifier_qualifiers),
                                        false,
                                    )?;

                                    if member_info.specifiers.storage_class.is_some() {
                                        return Err(ParseErrorKind::Misc(
                                            "Storage classes not supported here",
                                        )
                                        .at(declarator.source));
                                    }

                                    if member_info.specifiers.function_specifier.is_some() {
                                        return Err(ParseErrorKind::Misc(
                                            "Function specifiers cannot be used here",
                                        )
                                        .at(declarator.source));
                                    }

                                    fields.insert(
                                        member_info.name.clone(),
                                        ast::Field {
                                            ast_type: member_info.ast_type.clone(),
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

                ctx.ast_file.structs.push(ast::Struct {
                    name: name.clone(),
                    params: ast::TypeParams::default(),
                    fields,
                    is_packed,
                    source: composite.source,
                    privacy: Privacy::Private,
                });

                Ok(ast::TypeKind::Named(ast::NamePath::new_plain(name), vec![]))
            } else {
                Ok(ast::TypeKind::AnonymousStruct(ast::AnonymousStruct {
                    fields,
                    is_packed,
                }))
            }
        }
        CompositeKind::Union => {
            todo!("union composites")
        }
    }
}
