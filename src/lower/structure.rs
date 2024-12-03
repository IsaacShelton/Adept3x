use super::{builder::unpoly, datatype::lower_type, error::LowerError};
use crate::{ir, resolve::PolyRecipe, resolved};

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
                &ir_module.target,
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
    );

    Ok(())
}
