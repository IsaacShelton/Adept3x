use super::{resolve_expr, ResolveExprCtx};
use crate::{
    asg::{self, Member, TypedExpr},
    ast::{self, Privacy},
    resolve::{
        core_struct_info::{get_core_struct_info, CoreStructInfo},
        destination::resolve_expr_to_destination,
        error::{ResolveError, ResolveErrorKind},
        expr::ResolveExprMode,
        Initialized, PolyCatalog,
    },
    source_files::Source,
};

pub fn resolve_member_expr(
    ctx: &mut ResolveExprCtx,
    subject: &ast::Expr,
    field_name: &str,
    min_privacy: Privacy,
    source: Source,
) -> Result<TypedExpr, ResolveError> {
    let mut resolved_subject = resolve_expr(
        ctx,
        subject,
        None,
        Initialized::Require,
        ResolveExprMode::RequireValue,
    )?;

    while let asg::TypeKind::Ptr(inner) = &resolved_subject.ty.kind {
        resolved_subject = TypedExpr::new(
            inner.as_ref().clone(),
            asg::ExprKind::Dereference(Box::new(resolved_subject)).at(source),
        );
    }

    let CoreStructInfo {
        struct_ref,
        arguments,
        ..
    } = get_core_struct_info(ctx.asg, &resolved_subject.ty, source).map_err(|e| {
        e.unwrap_or_else(|| {
            ResolveErrorKind::CannotUseOperator {
                operator: ".".into(),
                on_type: resolved_subject.ty.to_string(),
            }
            .at(source)
        })
    })?;

    let structure = ctx
        .asg
        .structs
        .get(struct_ref)
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
        Privacy::Protected => {
            if min_privacy != Privacy::Protected && min_privacy != Privacy::Private {
                return Err(ResolveErrorKind::FieldIsPrivate {
                    field_name: field_name.to_string(),
                }
                .at(subject.source));
            }
        }
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
    assert!(structure.params.len() == arguments.len());
    for (name, argument) in structure.params.names().zip(arguments.iter()) {
        catalog
            .put_type(name, argument)
            .expect("unique polymorph name");
    }
    let ty = catalog
        .bake()
        .resolve_type(&found_field.ty)
        .map_err(ResolveError::from)?;

    let subject_destination = resolve_expr_to_destination(resolved_subject)?;

    Ok(TypedExpr::new(
        ty.clone(),
        asg::Expr::new(
            asg::ExprKind::Member(Box::new(Member {
                subject: subject_destination,
                struct_ref,
                index,
                field_type: ty,
            })),
            source,
        ),
    ))
}
