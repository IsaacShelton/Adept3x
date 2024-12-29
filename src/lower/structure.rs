use super::{
    builder::unpoly,
    datatype::{lower_type, ConcreteType},
    error::{LowerError, LowerErrorKind},
};
use crate::{
    asg::{self, Asg}, ir,
    resolve::{PolyCatalog, PolyRecipe},
    source_files::Source,
};

pub fn mono(
    ir_module: &ir::Module,
    asg: &Asg,
    resolved_structure_ref: asg::StructureRef,
    poly_recipe: PolyRecipe,
) -> Result<ir::StructureRef, LowerError> {
    let structure = asg
        .structures
        .get(resolved_structure_ref)
        .expect("referenced structure exists");

    let mut fields = Vec::with_capacity(structure.fields.len());

    for field in structure.fields.values() {
        fields.push(ir::Field {
            ir_type: lower_type(ir_module, &unpoly(&poly_recipe, &field.resolved_type)?, asg)?,
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
    resolved_structure_ref: asg::StructureRef,
    parameters: &[ConcreteType],
    asg: &Asg,
    source: Source,
) -> Result<ir::StructureRef, LowerError> {
    let structure = asg
        .structures
        .get(resolved_structure_ref)
        .expect("referenced structure to exist");

    if structure.parameters.len() != parameters.len() {
        return Err(LowerErrorKind::IncorrectNumberOfTypeArguments.at(source));
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
                mono(ir_module, asg, resolved_structure_ref, poly_recipe)
            })?;

    Ok(structure_ref)
}

pub fn lower_structure(
    ir_module: &mut ir::Module,
    structure_ref: asg::StructureRef,
    structure: &asg::Structure,
    asg: &Asg,
) -> Result<(), LowerError> {
    let mut fields = Vec::with_capacity(structure.fields.len());

    // NOTE: We only lower polymorphic structures on-demand, so skip them for now
    if !structure.parameters.parameters.is_empty() {
        return Ok(());
    }

    for field in structure.fields.values() {
        fields.push(ir::Field {
            ir_type: lower_type(
                ir_module,
                &unpoly(&PolyRecipe::default(), &field.resolved_type)?,
                asg,
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
