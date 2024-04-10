use super::{resolve_expr, ResolveExprCtx};
use crate::{
    ast::{self, Expr, Source},
    resolve::{
        conform_expr,
        error::{ResolveError, ResolveErrorKind},
        resolve_type, Initialized,
    },
    resolved::{self, TypedExpr},
};
use indexmap::IndexMap;

pub fn resolve_struct_literal_expr(
    ctx: &mut ResolveExprCtx<'_, '_>,
    ast_type: &ast::Type,
    fields: &IndexMap<String, Expr>,
    source: Source,
) -> Result<TypedExpr, ResolveError> {
    let resolved_type = resolve_type(
        ctx.type_search_ctx,
        ctx.resolved_ast.source_file_cache,
        ast_type,
    )?;

    let (name, structure_ref, memory_management) = match &resolved_type {
        resolved::Type::PlainOldData(name, structure_ref) => {
            (name, *structure_ref, resolved::MemoryManagement::None)
        }
        resolved::Type::ManagedStructure(name, structure_ref) => (
            name,
            *structure_ref,
            resolved::MemoryManagement::ReferenceCounted,
        ),
        _ => {
            return Err(ResolveError::new(
                ctx.resolved_ast.source_file_cache,
                ast_type.source,
                ResolveErrorKind::CannotCreateStructLiteralForNonPlainOldDataStructure {
                    bad_type: ast_type.to_string(),
                },
            ))
        }
    };

    let structure_type = resolved::Type::PlainOldData(name.clone(), structure_ref);
    let mut resolved_fields = IndexMap::new();

    for (name, value) in fields.iter() {
        let resolved_expr = resolve_expr(ctx, value, Initialized::Require)?;

        let structure = ctx
            .resolved_ast
            .structures
            .get(structure_ref)
            .expect("referenced structure to exist");

        let (index, _, field) = match structure.fields.get_full::<str>(&name) {
            Some(field) => field,
            None => {
                return Err(ResolveError::new(
                    ctx.resolved_ast.source_file_cache,
                    source,
                    ResolveErrorKind::FieldDoesNotExist {
                        field_name: name.to_string(),
                    },
                ))
            }
        };

        let resolved_expr = match conform_expr(&resolved_expr, &field.resolved_type) {
            Some(resolved_expr) => resolved_expr,
            None => {
                return Err(ResolveError::new(
                    ctx.resolved_ast.source_file_cache,
                    ast_type.source,
                    ResolveErrorKind::ExpectedTypeForField {
                        structure: ast_type.to_string(),
                        field_name: name.to_string(),
                        expected: field.resolved_type.to_string(),
                    },
                ))
            }
        };

        resolved_fields.insert(name.to_string(), (resolved_expr.expr, index));
    }

    let structure = ctx
        .resolved_ast
        .structures
        .get(structure_ref)
        .expect("referenced structure to exist");

    if resolved_fields.len() != structure.fields.len() {
        let missing = structure
            .fields
            .keys()
            .flat_map(|field_name| match resolved_fields.get(field_name) {
                None => Some(field_name.clone()),
                Some(_) => None,
            })
            .collect();

        return Err(ResolveError::new(
            ctx.resolved_ast.source_file_cache,
            source,
            ResolveErrorKind::MissingFields { fields: missing },
        ));
    }

    Ok(TypedExpr::new(
        resolved_type.clone(),
        resolved::Expr::new(
            resolved::ExprKind::StructureLiteral {
                structure_type,
                fields: resolved_fields,
                memory_management,
            },
            ast_type.source,
        ),
    ))
}
