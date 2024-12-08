use super::{resolve_expr, PreferredType, ResolveExprCtx};
use crate::{
    ast::{self, ConformBehavior, FieldInitializer, FillBehavior},
    resolve::{
        conform::{conform_expr, ConformMode, Perform},
        core_structure_info::get_core_structure_info,
        error::{ResolveError, ResolveErrorKind},
        Initialized, PolyCatalog, PolymorphError,
    },
    resolved::{self, StructLiteral, StructureRef, TypedExpr},
    source_files::Source,
};
use indexmap::IndexMap;
use itertools::Itertools;

#[derive(Clone, Debug)]
pub struct FieldInfo {
    index: usize,
    resolved_type: resolved::Type,
}

fn get_field_info<'a>(
    ctx: &'a ResolveExprCtx,
    structure_ref: StructureRef,
    arguments: &[resolved::Type],
    field_name: &str,
) -> Result<FieldInfo, PolymorphError> {
    let structure = ctx
        .resolved_ast
        .structures
        .get(structure_ref)
        .expect("referenced structure to exist");

    let (index, _, field) = structure
        .fields
        .get_full::<str>(field_name)
        .expect("referenced struct field to exist");

    let mut catalog = PolyCatalog::new();

    assert!(arguments.len() == structure.parameters.len());

    for (name, argument) in structure.parameters.names().zip(arguments.iter()) {
        catalog
            .put_type(name, argument)
            .expect("non-duplicate polymorphic type parameters for structure".into())
    }

    let recipe = catalog.bake();
    let resolved_type = recipe.resolve_type(&field.resolved_type)?;

    Ok(FieldInfo {
        index,
        resolved_type,
    })
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
    let (struct_name, structure_ref, parameters) =
        get_core_structure_info(ctx.resolved_ast, &resolved_struct_type, source)?;

    let struct_name = struct_name.clone();
    let parameters = parameters.to_vec();

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
        let field_info = get_field_info(ctx, structure_ref, &parameters, &field_name)
            .map_err(ResolveError::from)?;

        let mode = match conform_behavior {
            ConformBehavior::Adept(_) => ConformMode::Normal,
            ConformBehavior::C => ConformMode::Explicit,
        };

        let resolved_expr = conform_expr::<Perform>(
            ctx,
            &resolved_expr,
            &field_info.resolved_type,
            mode,
            conform_behavior,
            source,
        )
        .map_err(|_| {
            ResolveErrorKind::ExpectedTypeForField {
                structure: ast_type.to_string(),
                field_name: field_name.to_string(),
                expected: field_info.resolved_type.to_string(),
            }
            .at(ast_type.source)
        })?;

        if resolved_fields
            .insert(
                field_name.to_string(),
                (resolved_expr.expr, field_info.index),
            )
            .is_some()
        {
            return Err(ResolveErrorKind::FieldSpecifiedMoreThanOnce {
                struct_name: struct_name.to_string(),
                field_name: field_name.to_string(),
            }
            .at(ast_type.source));
        }

        next_index = field_info.index + 1;
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
                    let field_info = get_field_info(ctx, structure_ref, &parameters, field_name)
                        .map_err(ResolveError::from)?;

                    let zeroed =
                        resolved::ExprKind::Zeroed(Box::new(field_info.resolved_type.clone()))
                            .at(source);

                    resolved_fields.insert(field_name.to_string(), (zeroed, field_info.index));
                }
            }
        }

        assert_eq!(resolved_fields.len(), structure.fields.len());
    }

    let resolved_fields = resolved_fields
        .into_iter()
        .map(|(x, (y, z))| (x, y, z))
        .collect_vec();

    let structure_type =
        resolved::TypeKind::Structure(struct_name, structure_ref, parameters).at(source);

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
