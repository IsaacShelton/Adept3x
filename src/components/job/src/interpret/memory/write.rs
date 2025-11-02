use super::{
    super::{error::InterpreterError, size_of::size_of, value::Value},
    Memory, ir,
};
use crate::interpret::value::{StructLiteral, Tainted, ValueKind};

// TODO: Clean up and reduce redundancy
impl Memory {
    pub fn write<'env>(
        &mut self,
        destination: u64,
        value: Value<'env>,
        ir_module: &'env ir::Ir<'env>,
    ) -> Result<(), InterpreterError> {
        if self.is_reserved_address(destination) {
            return Err(InterpreterError::SegfaultWrite);
        }

        match value.kind {
            ValueKind::Undefined => Ok(()),
            ValueKind::Literal(literal) => {
                self.write_literal(destination, literal, value.tainted, ir_module)
            }
            ValueKind::StructLiteral(literal) => {
                self.write_struct_literal(destination, literal, ir_module)
            }
        }
    }

    fn write_struct_literal<'env>(
        &mut self,
        destination: u64,
        literal: StructLiteral<'env>,
        ir_module: &'env ir::Ir<'env>,
    ) -> Result<(), InterpreterError> {
        let mut offset = destination;

        for (field, value) in literal.fields.iter().zip(literal.values.iter()) {
            let size = size_of(&field.ir_type, ir_module);
            self.write(offset, value.clone(), ir_module)?;
            offset += size;
        }

        Ok(())
    }

    fn write_literal<'env>(
        &mut self,
        destination: u64,
        literal: ir::Literal<'env>,
        tainted: Option<Tainted>,
        ir_module: &'env ir::Ir<'env>,
    ) -> Result<(), InterpreterError> {
        match literal {
            ir::Literal::Void => (),
            ir::Literal::Boolean(x) => self.write_bytes(destination, &[x.into()], tainted),
            ir::Literal::Integer(immediate) => {
                self.write_bytes(destination, &immediate.to_le_bytes(), tainted)
            }
            ir::Literal::Float32(x) => self.write_bytes(destination, &x.to_le_bytes(), tainted),
            ir::Literal::Float64(x) => self.write_bytes(destination, &x.to_le_bytes(), tainted),
            ir::Literal::NullTerminatedString(value) => {
                // TODO: Cache the allocation
                let string_bytes = value.to_bytes_with_nul();
                let alloced = self.alloc_permanent_raw(string_bytes.len().try_into().unwrap());
                self.write_bytes(alloced, string_bytes, tainted);
                self.write_bytes(destination, &alloced.to_le_bytes(), tainted);
            }
            ir::Literal::Zeroed(ty) => {
                let size = size_of(&ty, ir_module);

                if self.is_heap_address(destination) {
                    for i in 0..size {
                        self.heap[(destination - Self::HEAP_OFFSET + i) as usize] = 0;
                    }
                } else {
                    for i in 0..size {
                        self.stack[(destination - Self::STACK_OFFSET + i) as usize] = 0;
                    }
                }
            }
        }

        Ok(())
    }

    fn write_bytes(&mut self, destination: u64, bytes: &[u8], tainted: Option<Tainted>) {
        if self.is_heap_address(destination) {
            self.write_bytes_heap(destination, bytes, tainted);
        } else {
            self.write_bytes_stack(destination, bytes, tainted);
        }
    }

    fn write_bytes_heap(&mut self, destination: u64, bytes: &[u8], tainted: Option<Tainted>) {
        let start_index = destination as usize - Self::HEAP_OFFSET as usize;

        self.heap
            .iter_mut()
            .skip(start_index)
            .zip(bytes.iter().copied())
            .for_each(|(memory, byte)| *memory = byte);

        self.heap_tainted_by_comptime_sizeof
            .iter_mut()
            .skip(start_index)
            .take(bytes.len())
            .for_each(|mut x| {
                *x = tainted == Some(Tainted::ByCompilationHostSizeof);
            });
    }

    fn write_bytes_stack(&mut self, destination: u64, bytes: &[u8], tainted: Option<Tainted>) {
        let start_index = destination as usize - Self::STACK_OFFSET as usize;

        self.stack
            .iter_mut()
            .skip(start_index)
            .zip(bytes.iter().copied())
            .for_each(|(memory, byte)| *memory = byte);

        self.stack_tainted_by_comptime_sizeof
            .iter_mut()
            .skip(start_index)
            .take(bytes.len())
            .for_each(|mut x| {
                *x = tainted == Some(Tainted::ByCompilationHostSizeof);
            });
    }
}
