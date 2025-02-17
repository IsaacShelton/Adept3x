use super::{
    builder::unpoly,
    datatype::{lower_type, ConcreteType},
    error::{LowerError, LowerErrorKind},
};
use crate::{
    asg::{self, Asg},
    ir,
    resolve::{PolyCatalog, PolyRecipe},
    source_files::Source,
};

pub fn mono(
    ir_module: &ir::Module,
    asg: &Asg,
    struct_ref: asg::StructRef,
    poly_recipe: PolyRecipe,
) -> Result<ir::StructRef, LowerError> {
    let structure = asg
        .structs
        .get(struct_ref)
        .expect("referenced structure exists");

    let mut fields = Vec::with_capacity(structure.fields.len());

    for field in structure.fields.values() {
        fields.push(ir::Field {
            ir_type: lower_type(ir_module, &unpoly(&poly_recipe, &field.ty)?, asg)?,
            properties: ir::FieldProperties::default(),
            source: field.source,
        });
    }

    let struct_ref = ir_module.structs.insert(
        struct_ref,
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

pub fn monomorphize_structure(
    ir_module: &ir::Module,
    struct_ref: asg::StructRef,
    parameters: &[ConcreteType],
    asg: &Asg,
    source: Source,
) -> Result<ir::StructRef, LowerError> {
    let structure = asg
        .structs
        .get(struct_ref)
        .expect("referenced structure to exist");

    if structure.params.len() != parameters.len() {
        return Err(LowerErrorKind::IncorrectNumberOfTypeArguments.at(source));
    }

    let mut catalog = PolyCatalog::new();

    for (name, concrete_type) in structure.params.names().zip(parameters.iter()) {
        eprintln!("TODO: Ensure that type arguments satisfy constraints");

        catalog
            .put_type(name, &concrete_type.0)
            .expect("no duplicate names");
    }

    let poly_recipe = catalog.bake();

    let struct_ref = ir_module
        .structs
        .translate(struct_ref, poly_recipe, |poly_recipe| {
            mono(ir_module, asg, struct_ref, poly_recipe)
        })?;

    Ok(struct_ref)
}

pub fn lower_struct(
    ir_module: &mut ir::Module,
    struct_ref: asg::StructRef,
    structure: &asg::Struct,
    asg: &Asg,
) -> Result<(), LowerError> {
    let mut fields = Vec::with_capacity(structure.fields.len());

    // NOTE: We only lower polymorphic structures on-demand, so skip them for now
    if !structure.params.is_empty() {
        return Ok(());
    }

    for field in structure.fields.values() {
        fields.push(ir::Field {
            ir_type: lower_type(ir_module, &unpoly(&PolyRecipe::default(), &field.ty)?, asg)?,
            properties: ir::FieldProperties::default(),
            source: field.source,
        });
    }

    ir_module.structs.insert(
        struct_ref,
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
