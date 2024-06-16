use super::{resolve_expr, PreferredType, ResolveExprCtx};
use crate::{
    ast::{self, Expr, FillBehavior, Source},
    resolve::{
        conform_expr,
        core_structure_info::get_core_structure_info,
        error::{ResolveError, ResolveErrorKind},
        resolve_type, ConformMode, Initialized,
    },
    resolved::{self, StructureRef, TypedExpr},
};
use indexmap::IndexMap;
use itertools::Itertools;

fn get_field_info<'a>(
    ctx: &'a ResolveExprCtx<'_, '_>,
    structure_ref: StructureRef,
    field_name: &str,
) -> (usize, &'a resolved::Field) {
    let (index, _, field) = ctx
        .resolved_ast
        .structures
        .get(structure_ref)
        .expect("referenced structure to exist")
        .fields
        .get_full::<str>(field_name)
        .expect("referenced struct field to exist");
    (index, field)
}

pub fn resolve_struct_literal_expr(
    ctx: &mut ResolveExprCtx<'_, '_>,
    ast_type: &ast::Type,
    fields: &IndexMap<String, Expr>,
    fill_behavior: FillBehavior,
    source: Source,
) -> Result<TypedExpr, ResolveError> {
    let resolved_type = resolve_type(
        ctx.type_search_ctx,
        ctx.resolved_ast.source_file_cache,
        ast_type,
        &mut Default::default(),
    )?;

    let (name, structure_ref, memory_management) = get_core_structure_info(&resolved_type, source)?;

    let structure_type =
        resolved::TypeKind::PlainOldData(name.to_string(), structure_ref).at(source);

    let mut resolved_fields = IndexMap::new();

    for (name, value) in fields.iter() {
        // Ensure field exists on structure
        {
            let structure = ctx
                .resolved_ast
                .structures
                .get(structure_ref)
                .expect("referenced structure to exist");

            if !structure.fields.contains_key::<str>(&name) {
                return Err(ResolveErrorKind::FieldDoesNotExist {
                    field_name: name.to_string(),
                }
                .at(source));
            }
        }

        // Resolve expression value given for this field
        let resolved_expr = resolve_expr(
            ctx,
            value,
            Some(PreferredType::FieldType(structure_ref, &name)),
            Initialized::Require,
        )?;

        // Lookup additional details required for resolution
        let (index, field) = get_field_info(ctx, structure_ref, &name);

        let resolved_expr = conform_expr(
            &resolved_expr,
            &field.resolved_type,
            ConformMode::Normal,
            source,
        )
        .ok_or_else(|| {
            ResolveErrorKind::ExpectedTypeForField {
                structure: ast_type.to_string(),
                field_name: name.to_string(),
                expected: field.resolved_type.to_string(),
            }
            .at(ast_type.source)
        })?;

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
                None => Some(field_name.as_str()),
                Some(_) => None,
            })
            .collect_vec();

        match fill_behavior {
            FillBehavior::Forbid => {
                let missing = missing.iter().map(ToString::to_string).collect_vec();
                return Err(ResolveErrorKind::MissingFields { fields: missing }.at(source));
            }
            FillBehavior::Zeroed => {
                for field_name in missing.iter() {
                    let (index, field) = get_field_info(ctx, structure_ref, field_name);
                    let zeroed = resolved::ExprKind::Zeroed(field.resolved_type.clone()).at(source);
                    resolved_fields.insert(field_name.to_string(), (zeroed, index));
                }
            }
        }

        assert_eq!(resolved_fields.len(), structure.fields.len());
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
