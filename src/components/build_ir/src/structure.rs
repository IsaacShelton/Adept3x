use super::{
    datatype::{ConcreteType, lower_type},
    error::{LowerError, LowerErrorKind},
    func_builder::unpoly,
};
use crate::ModBuilder;
use asg::{PolyCatalog, PolyRecipe};
use source_files::Source;

pub fn monomorphize_struct(
    mod_builder: &ModBuilder,
    asg_struct_ref: asg::StructRef,
    poly_recipe: PolyRecipe,
) -> Result<ir::StructRef, LowerError> {
    let structure = mod_builder
        .asg
        .structs
        .get(asg_struct_ref)
        .expect("referenced structure exists");

    let mut fields = Vec::with_capacity(structure.fields.len());

    for field in structure.fields.values() {
        fields.push(ir::Field {
            ir_type: lower_type(mod_builder, &unpoly(&poly_recipe, &field.ty)?)?,
            properties: ir::FieldProperties::default(),
            source: field.source,
        });
    }

    let struct_ref = mod_builder.structs.insert(
        asg_struct_ref,
        ir::Struct {
            name: Some(structure.name.plain().to_string()),
            fields,
            is_packed: structure.is_packed,
            source: structure.source,
        },
        poly_recipe,
    );

    Ok(struct_ref)
}

pub fn monomorphize_struct_with_params(
    mod_builder: &ModBuilder,
    asg_struct_ref: asg::StructRef,
    parameters: &[ConcreteType],
    source: Source,
) -> Result<ir::StructRef, LowerError> {
    let structure = mod_builder
        .asg
        .structs
        .get(asg_struct_ref)
        .expect("referenced structure to exist");

    if structure.params.len() != parameters.len() {
        return Err(LowerErrorKind::IncorrectNumberOfTypeArguments.at(source));
    }

    let mut catalog = PolyCatalog::new();

    for (name, concrete_type) in structure.params.names().zip(parameters.iter()) {
        catalog
            .put_type(name, &concrete_type.0)
            .expect("no duplicate names");
    }

    let poly_recipe = catalog.bake();

    let ir_struct_ref =
        mod_builder
            .structs
            .translate(asg_struct_ref, poly_recipe, |poly_recipe| {
                monomorphize_struct(mod_builder, asg_struct_ref, poly_recipe)
            })?;

    Ok(ir_struct_ref)
}

pub fn lower_struct(
    mod_builder: &mut ModBuilder,
    asg_struct_ref: asg::StructRef,
) -> Result<(), LowerError> {
    let structure = mod_builder.asg.structs.get(asg_struct_ref).unwrap();
    let mut fields = Vec::with_capacity(structure.fields.len());

    // NOTE: We only lower polymorphic structures on-demand, so skip them for now
    if !structure.params.is_empty() {
        return Ok(());
    }

    for field in structure.fields.values() {
        fields.push(ir::Field {
            ir_type: lower_type(mod_builder, &unpoly(&PolyRecipe::default(), &field.ty)?)?,
            properties: ir::FieldProperties::default(),
            source: field.source,
        });
    }

    mod_builder.structs.insert(
        asg_struct_ref,
        ir::Struct {
            name: Some(structure.name.plain().to_string()),
            fields,
            is_packed: structure.is_packed,
            source: structure.source,
        },
        PolyRecipe::default(),
    );

    Ok(())
}
