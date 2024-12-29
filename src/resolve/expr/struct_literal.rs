use super::{resolve_expr, PreferredType, ResolveExprCtx};
use crate::{
    asg::{self, StructLiteral, StructRef, TypedExpr},
    ast::{self, ConformBehavior, FieldInitializer, FillBehavior},
    resolve::{
        conform::{conform_expr, ConformMode, Perform},
        core_struct_info::{get_core_struct_info, CoreStructInfo},
        error::{ResolveError, ResolveErrorKind},
        Initialized, PolyCatalog, PolymorphError,
    },
    source_files::Source,
};
use indexmap::IndexMap;
use itertools::Itertools;

#[derive(Clone, Debug)]
pub struct FieldInfo {
    pub index: usize,
    pub ty: asg::Type,
}

fn get_field_info<'a>(
    ctx: &'a ResolveExprCtx,
    struct_ref: StructRef,
    arguments: &[asg::Type],
    field_name: &str,
) -> Result<FieldInfo, PolymorphError> {
    let structure = ctx
        .asg
        .structs
        .get(struct_ref)
        .expect("referenced structure to exist");

    let (index, _name, field) = structure
        .fields
        .get_full::<str>(field_name)
        .expect("referenced struct field to exist");

    let mut catalog = PolyCatalog::new();
    assert!(arguments.len() == structure.params.len());

    for (name, argument) in structure.params.names().zip(arguments.iter()) {
        catalog
            .put_type(name, argument)
            .expect("non-duplicate polymorphic type parameters for structure")
    }

    let ty = catalog.bake().resolve_type(&field.ty)?;

    Ok(FieldInfo { index, ty })
}

pub fn resolve_struct_literal_expr(
    ctx: &mut ResolveExprCtx,
    ast_type: &ast::Type,
    fields: &[FieldInitializer],
    fill_behavior: FillBehavior,
    conform_behavior: ConformBehavior,
    source: Source,
) -> Result<TypedExpr, ResolveError> {
    let struct_type = ctx.type_ctx().resolve(ast_type)?;

    let CoreStructInfo {
        name: struct_name,
        struct_ref,
        arguments,
    } = get_core_struct_info(ctx.asg, &struct_type, source).map_err(|e| {
        e.unwrap_or_else(|| {
            ResolveErrorKind::CannotCreateStructLiteralForNonStructure {
                bad_type: struct_type.to_string(),
            }
            .at(struct_type.source)
        })
    })?;

    let struct_name = struct_name.clone();
    let arguments = arguments.to_vec();

    let mut next_index = 0;
    let mut resolved_fields = IndexMap::new();

    for field_initializer in fields.iter() {
        let all_fields = &ctx
            .asg
            .structs
            .get(struct_ref)
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
                .asg
                .structs
                .get(struct_ref)
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
            Some(PreferredType::FieldType(struct_ref, &field_name)),
            Initialized::Require,
        )?;

        // Lookup additional details required for resolution
        let field_info =
            get_field_info(ctx, struct_ref, &arguments, &field_name).map_err(ResolveError::from)?;

        let mode = match conform_behavior {
            ConformBehavior::Adept(_) => ConformMode::Normal,
            ConformBehavior::C => ConformMode::Explicit,
        };

        let resolved_expr = conform_expr::<Perform>(
            ctx,
            &resolved_expr,
            &field_info.ty,
            mode,
            conform_behavior,
            source,
        )
        .map_err(|_| {
            ResolveErrorKind::ExpectedTypeForField {
                structure: ast_type.to_string(),
                field_name: field_name.to_string(),
                expected: field_info.ty.to_string(),
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
        .asg
        .structs
        .get(struct_ref)
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
                    let field_info = get_field_info(ctx, struct_ref, &arguments, field_name)
                        .map_err(ResolveError::from)?;

                    let zeroed = asg::ExprKind::Zeroed(Box::new(field_info.ty.clone())).at(source);

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

    let struct_type = asg::TypeKind::Structure(struct_name, struct_ref, arguments).at(source);

    Ok(TypedExpr::new(
        struct_type.clone(),
        asg::Expr::new(
            asg::ExprKind::StructLiteral(Box::new(StructLiteral {
                struct_type,
                fields: resolved_fields,
            })),
            ast_type.source,
        ),
    ))
}
