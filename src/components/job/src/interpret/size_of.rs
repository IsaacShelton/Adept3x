use crate::ir;

pub fn size_of<'env>(ir_type: &ir::Type<'env>, ir_module: &ir::Ir<'env>) -> u64 {
    match ir_type {
        ir::Type::Ptr(_) => 8,
        ir::Type::Bool => 1,
        ir::Type::I(int_bits, _) => int_bits.bytes().bytes(),
        ir::Type::F(float_bits) => float_bits.bytes().bytes(),
        ir::Type::Void => 0,
        ir::Type::Union(_) => todo!("interpreter write union"),
        ir::Type::Struct(struct_ref) => {
            let structure = &ir_module.structs[*struct_ref];

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
        ir::Type::FuncPtr => todo!("size_of ir::FuncPtr"),
        ir::Type::FixedArray(_) => todo!("size_of ir::FixedArray"),
        ir::Type::Vector(_) => todo!("interpreting vector types not supported yet"),
        ir::Type::Complex(_) => todo!("interpreting complex numeric types not support yet"),
        ir::Type::Atomic(inner) => size_of(inner, ir_module),
        ir::Type::IncompleteArray(_) => 8,
    }
}
