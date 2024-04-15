use super::{resolve_expr, ResolveExprCtx};
use crate::{
    ast::{self, Source},
    resolve::{
        core_structure_info::get_core_structure_info, destination::resolve_expr_to_destination, error::{ResolveError, ResolveErrorKind}, Initialized
    },
    resolved::{self, TypedExpr},
};

pub fn resolve_member_expr(
    ctx: &mut ResolveExprCtx<'_, '_>,
    subject: &ast::Expr,
    field_name: &str,
    source: Source,
) -> Result<TypedExpr, ResolveError> {
    let resolved_subject = resolve_expr(ctx, subject, None, Initialized::Require)?;

    let (_, structure_ref, memory_management) = get_core_structure_info(
        ctx.resolved_ast.source_file_cache,
        &resolved_subject.resolved_type,
        source,
    )?;

    let structure = ctx
        .resolved_ast
        .structures
        .get(structure_ref)
        .expect("referenced struct to exist");

    let (index, _key, found_field) = match structure.fields.get_full(field_name) {
        Some(found) => found,
        None => {
            return Err(ResolveError::new(
                ctx.resolved_ast.source_file_cache,
                subject.source,
                ResolveErrorKind::FieldDoesNotExist {
                    field_name: field_name.to_string(),
                },
            ))
        }
    };

    match found_field.privacy {
        resolved::Privacy::Public => (),
        resolved::Privacy::Private => {
            return Err(ResolveError::new(
                ctx.resolved_ast.source_file_cache,
                subject.source,
                ResolveErrorKind::FieldIsPrivate {
                    field_name: field_name.to_string(),
                },
            ))
        }
    }

    let subject_destination =
        resolve_expr_to_destination(ctx.resolved_ast.source_file_cache, resolved_subject)?;

    Ok(TypedExpr::new(
        found_field.resolved_type.clone(),
        resolved::Expr::new(
            resolved::ExprKind::Member {
                subject: subject_destination,
                structure_ref,
                index,
                field_type: found_field.resolved_type.clone(),
                memory_management,
            },
            source,
        ),
    ))
}
