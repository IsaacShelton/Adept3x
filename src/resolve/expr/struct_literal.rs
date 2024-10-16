use super::{resolve_expr, PreferredType, ResolveExprCtx};
use crate::{
    ast::{self, ConformBehavior, FieldInitializer, FillBehavior},
    resolve::{
        conform::{conform_expr, ConformMode, Perform},
        core_structure_info::get_core_structure_info,
        error::{ResolveError, ResolveErrorKind},
        Initialized,
    },
    resolved::{self, StructLiteral, StructureRef, TypedExpr},
    source_files::Source,
};
use indexmap::IndexMap;
use itertools::Itertools;

fn get_field_info<'a>(
    ctx: &'a ResolveExprCtx,
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
    ctx: &mut ResolveExprCtx,
    ast_type: &ast::Type,
    fields: &[FieldInitializer],
    fill_behavior: FillBehavior,
    conform_behavior: ConformBehavior,
    source: Source,
) -> Result<TypedExpr, ResolveError> {
    let resolved_struct_type = ctx.type_ctx().resolve(ast_type)?;
    let (struct_name, structure_ref) =
        get_core_structure_info(ctx.resolved_ast, &resolved_struct_type, source)?;

    let structure_type =
        resolved::TypeKind::Structure(struct_name.clone(), structure_ref).at(source);

    let mut next_index = 0;
    let mut resolved_fields = IndexMap::new();

    for field_initializer in fields.iter() {
        let all_fields = &ctx
            .resolved_ast
            .structures
            .get(structure_ref)
            .expect("referenced struct to exist")
            .fields;

        let field_name = match field_initializer
            .name
            .as_ref()
            .or_else(|| all_fields.get_index(next_index).map(|(k, _v)| k))
            .cloned()
        {
            Some(field_name) => field_name,
            None => return Err(ResolveErrorKind::OutOfFields.at(source)),
        };

        // Ensure field exists on structure
        {
            let structure = ctx
                .resolved_ast
                .structures
                .get(structure_ref)
                .expect("referenced structure to exist");

            if !structure.fields.contains_key::<str>(&field_name) {
                return Err(ResolveErrorKind::FieldDoesNotExist {
                    field_name: field_name.to_string(),
                }
                .at(source));
            }
        }

        // Resolve expression value given for this field
        let resolved_expr = resolve_expr(
            ctx,
            &field_initializer.value,
            Some(PreferredType::FieldType(structure_ref, &field_name)),
            Initialized::Require,
        )?;

        // Lookup additional details required for resolution
        let (index, field) = get_field_info(ctx, structure_ref, &field_name);

        let mode = match conform_behavior {
            ConformBehavior::Adept(_) => ConformMode::Normal,
            ConformBehavior::C => ConformMode::Explicit,
        };

        let resolved_expr = conform_expr::<Perform>(
            ctx,
            &resolved_expr,
            &field.resolved_type,
            mode,
            conform_behavior,
            source,
        )
        .map_err(|_| {
            ResolveErrorKind::ExpectedTypeForField {
                structure: ast_type.to_string(),
                field_name: field_name.to_string(),
                expected: field.resolved_type.to_string(),
            }
            .at(ast_type.source)
        })?;

        if resolved_fields
            .insert(field_name.to_string(), (resolved_expr.expr, index))
            .is_some()
        {
            let (struct_name, _) =
                get_core_structure_info(ctx.resolved_ast, &resolved_struct_type, source)?;

            return Err(ResolveErrorKind::FieldSpecifiedMoreThanOnce {
                struct_name: struct_name.to_string(),
                field_name: field_name.to_string(),
            }
            .at(ast_type.source));
        }

        next_index = index + 1;
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
                    let zeroed = resolved::ExprKind::Zeroed(Box::new(field.resolved_type.clone()))
                        .at(source);
                    resolved_fields.insert(field_name.to_string(), (zeroed, index));
                }
            }
        }

        assert_eq!(resolved_fields.len(), structure.fields.len());
    }

    Ok(TypedExpr::new(
        resolved_struct_type.clone(),
        resolved::Expr::new(
            resolved::ExprKind::StructLiteral(Box::new(StructLiteral {
                structure_type,
                fields: resolved_fields,
            })),
            ast_type.source,
        ),
    ))
}
