use super::{error::LowerError, lower_type};
use crate::{ir, resolved};

pub fn lower_structure(
    ir_module: &mut ir::Module,
    structure_ref: resolved::StructureRef,
    structure: &resolved::Structure,
    resolved_ast: &resolved::Ast,
) -> Result<(), LowerError> {
    let mut fields = Vec::with_capacity(structure.fields.len());

    for field in structure.fields.values() {
        fields.push(ir::Field {
            ir_type: lower_type(&ir_module.target, &field.resolved_type, resolved_ast)?,
            properties: ir::FieldProperties::default(),
            source: field.source,
        });
    }

    ir_module.structures.insert(
        structure_ref,
        ir::Structure {
            fields,
            is_packed: structure.is_packed,
            source: structure.source,
        },
    );

    Ok(())
}
