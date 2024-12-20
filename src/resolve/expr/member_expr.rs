use super::{resolve_expr, ResolveExprCtx};
use crate::{
    ast::{self, Privacy},
    resolve::{
        core_structure_info::{get_core_structure_info, CoreStructInfo},
        destination::resolve_expr_to_destination,
        error::{ResolveError, ResolveErrorKind},
        Initialized, PolyCatalog,
    },
    resolved::{self, Member, TypedExpr},
    source_files::Source,
};

pub fn resolve_member_expr(
    ctx: &mut ResolveExprCtx,
    subject: &ast::Expr,
    field_name: &str,
    min_privacy: Privacy,
    source: Source,
) -> Result<TypedExpr, ResolveError> {
    let resolved_subject = resolve_expr(ctx, subject, None, Initialized::Require)?;

    let CoreStructInfo {
        structure_ref,
        arguments,
        ..
    } = get_core_structure_info(ctx.resolved_ast, &resolved_subject.resolved_type, source)
        .map_err(|e| {
            e.unwrap_or_else(|| {
                ResolveErrorKind::CannotUseOperator {
                    operator: ".".into(),
                    on_type: resolved_subject.resolved_type.to_string(),
                }
                .at(source)
            })
        })?;

    let structure = ctx
        .resolved_ast
        .structures
        .get(structure_ref)
        .expect("referenced struct to exist");

    let (index, _key, found_field) = match structure.fields.get_full(field_name) {
        Some(found) => found,
        None => {
            return Err(ResolveErrorKind::FieldDoesNotExist {
                field_name: field_name.to_string(),
            }
            .at(subject.source))
        }
    };

    match found_field.privacy {
        Privacy::Public => (),
        Privacy::Private => {
            if min_privacy != Privacy::Private {
                return Err(ResolveErrorKind::FieldIsPrivate {
                    field_name: field_name.to_string(),
                }
                .at(subject.source));
            }
        }
    }

    let mut catalog = PolyCatalog::new();
    assert!(structure.parameters.len() == arguments.len());
    for (name, argument) in structure.parameters.names().zip(arguments.iter()) {
        catalog
            .put_type(name, argument)
            .expect("unique polymorph name");
    }
    let resolved_type = catalog
        .bake()
        .resolve_type(&found_field.resolved_type)
        .map_err(ResolveError::from)?;

    let subject_destination = resolve_expr_to_destination(resolved_subject)?;

    Ok(TypedExpr::new(
        resolved_type.clone(),
        resolved::Expr::new(
            resolved::ExprKind::Member(Box::new(Member {
                subject: subject_destination,
                structure_ref,
                index,
                field_type: resolved_type,
            })),
            source,
        ),
    ))
}
