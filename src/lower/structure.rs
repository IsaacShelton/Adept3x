use super::{
    builder::unpoly,
    datatype::{lower_type, ConcreteType},
    error::{LowerError, LowerErrorKind},
};
use crate::{
    ir,
    resolve::{PolyCatalog, PolyRecipe},
    resolved,
    source_files::Source,
};

pub fn mono(
    ir_module: &ir::Module,
    resolved_ast: &resolved::Ast,
    resolved_structure_ref: resolved::StructureRef,
    poly_recipe: PolyRecipe,
) -> Result<ir::StructureRef, LowerError> {
    let structure = resolved_ast
        .structures
        .get(resolved_structure_ref)
        .expect("referenced structure exists");

    let mut fields = Vec::with_capacity(structure.fields.len());

    for field in structure.fields.values() {
        fields.push(ir::Field {
            ir_type: lower_type(
                ir_module,
                &unpoly(&poly_recipe, &field.resolved_type)?,
                resolved_ast,
            )?,
            properties: ir::FieldProperties::default(),
            source: field.source,
        });
    }

    let structure_ref = ir_module.structures.insert(
        resolved_structure_ref,
        ir::Structure {
            name: Some(structure.name.plain().to_string()),
            fields,
            is_packed: structure.is_packed,
            source: structure.source,
        },
        poly_recipe,
    );

    Ok(structure_ref)
}

pub fn monomorphize_structure(
    ir_module: &ir::Module,
    resolved_structure_ref: resolved::StructureRef,
    parameters: &[ConcreteType],
    resolved_ast: &resolved::Ast,
    source: Source,
) -> Result<ir::StructureRef, LowerError> {
    let structure = resolved_ast
        .structures
        .get(resolved_structure_ref)
        .expect("referenced structure to exist");

    if structure.parameters.len() != parameters.len() {
        return Err(LowerErrorKind::MismatchedTypeParameterLengths.at(source));
    }

    let mut catalog = PolyCatalog::new();

    for (name, concrete_type) in structure.parameters.names().zip(parameters.iter()) {
        eprintln!("TODO: Ensure that type arguments satisfy constraints");

        catalog
            .put_type(name, &concrete_type.0)
            .expect("no duplicate names");
    }

    let poly_recipe = catalog.bake();

    let structure_ref =
        ir_module
            .structures
            .translate(resolved_structure_ref, poly_recipe, |poly_recipe| {
                mono(ir_module, resolved_ast, resolved_structure_ref, poly_recipe)
            })?;

    Ok(structure_ref)
}

pub fn lower_structure(
    ir_module: &mut ir::Module,
    structure_ref: resolved::StructureRef,
    structure: &resolved::Structure,
    resolved_ast: &resolved::Ast,
) -> Result<(), LowerError> {
    let mut fields = Vec::with_capacity(structure.fields.len());

    if !structure.parameters.parameters.is_empty() {
        eprintln!("warning: lowering generic type parameters is not supported yet, skipping...");
        return Ok(());
    }

    for field in structure.fields.values() {
        fields.push(ir::Field {
            ir_type: lower_type(
                ir_module,
                &unpoly(&PolyRecipe::default(), &field.resolved_type)?,
                resolved_ast,
            )?,
            properties: ir::FieldProperties::default(),
            source: field.source,
        });
    }

    ir_module.structures.insert(
        structure_ref,
        ir::Structure {
            name: Some(structure.name.plain().to_string()),
            fields,
            is_packed: structure.is_packed,
            source: structure.source,
        },
        PolyRecipe::default(),
    );

    Ok(())
}
