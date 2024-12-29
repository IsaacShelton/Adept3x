use crate::ir;

pub fn size_of(ir_type: &ir::Type, ir_module: &ir::Module) -> u64 {
    match ir_type {
        ir::Type::Pointer(_) => 8,
        ir::Type::Boolean => 1,
        ir::Type::S8 => 1,
        ir::Type::S16 => 2,
        ir::Type::S32 => 4,
        ir::Type::S64 => 8,
        ir::Type::U8 => 1,
        ir::Type::U16 => 2,
        ir::Type::U32 => 4,
        ir::Type::U64 => 8,
        ir::Type::F32 => 4,
        ir::Type::F64 => 8,
        ir::Type::Void => 0,
        ir::Type::Union(_) => todo!("interpreter write union"),
        ir::Type::Structure(struct_ref) => {
            let structure = ir_module.structs.get(*struct_ref);

            // NOTE: We don't do alignment in the interpreter
            structure
                .fields
                .iter()
                .fold(0, |acc, field| acc + size_of(&field.ir_type, ir_module))
        }
        ir::Type::AnonymousComposite(composite) => {
            // NOTE: We don't do alignment in the interpreter
            composite
                .fields
                .iter()
                .fold(0, |acc, field| acc + size_of(&field.ir_type, ir_module))
        }
        ir::Type::FunctionPointer => todo!(),
        ir::Type::FixedArray(_) => todo!(),
        ir::Type::Vector(_) => todo!("interpreting vector types not supported yet"),
        ir::Type::Complex(_) => todo!("interpreting complex numeric types not support yet"),
        ir::Type::Atomic(inner) => size_of(inner, ir_module),
        ir::Type::IncompleteArray(_) => 8,
    }
}
