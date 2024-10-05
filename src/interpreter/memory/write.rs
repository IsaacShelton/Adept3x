use super::{
    super::{error::InterpreterError, size_of::size_of, value::Value},
    Memory,
};
use crate::{interpreter::value::StructLiteral, ir};

impl Memory {
    pub fn write(
        &mut self,
        destination: u64,
        value: Value<'_>,
        ir_module: &ir::Module,
    ) -> Result<(), InterpreterError> {
        if self.is_reserved_address(destination) {
            return Err(InterpreterError::SegfaultWrite);
        }

        match value {
            Value::Undefined => Ok(()),
            Value::Literal(literal) => self.write_literal(destination, literal, ir_module),
            Value::StructLiteral(literal) => {
                self.write_struct_literal(destination, literal, ir_module)
            }
        }
    }

    fn write_struct_literal(
        &mut self,
        destination: u64,
        literal: StructLiteral,
        ir_module: &ir::Module,
    ) -> Result<(), InterpreterError> {
        let mut offset = destination;

        for (field, value) in literal.fields.iter().zip(literal.values.iter()) {
            let size = size_of(&field.ir_type, ir_module);
            self.write(offset, value.clone(), ir_module)?;
            offset += size;
        }

        Ok(())
    }

    fn write_literal(
        &mut self,
        destination: u64,
        literal: ir::Literal,
        ir_module: &ir::Module,
    ) -> Result<(), InterpreterError> {
        match literal {
            ir::Literal::Void => (),
            ir::Literal::Boolean(x) => self.write_bytes(destination, &[x.into()]),
            ir::Literal::Signed8(x) => self.write_bytes(destination, &x.to_le_bytes()),
            ir::Literal::Unsigned8(x) => self.write_bytes(destination, &x.to_le_bytes()),
            ir::Literal::Signed16(x) => self.write_bytes(destination, &x.to_le_bytes()),
            ir::Literal::Unsigned16(x) => self.write_bytes(destination, &x.to_le_bytes()),
            ir::Literal::Signed32(x) => self.write_bytes(destination, &x.to_le_bytes()),
            ir::Literal::Unsigned32(x) => self.write_bytes(destination, &x.to_le_bytes()),
            ir::Literal::Signed64(x) => self.write_bytes(destination, &x.to_le_bytes()),
            ir::Literal::Unsigned64(x) => self.write_bytes(destination, &x.to_le_bytes()),
            ir::Literal::Float32(x) => self.write_bytes(destination, &x.to_le_bytes()),
            ir::Literal::Float64(x) => self.write_bytes(destination, &x.to_le_bytes()),
            ir::Literal::NullTerminatedString(value) => {
                let string_bytes = value.as_bytes_with_nul();
                let alloced = self.alloc_permanent_raw(string_bytes.len().try_into().unwrap());
                self.write_bytes(alloced, string_bytes);
                self.write_bytes(destination, &alloced.to_le_bytes());
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

    fn write_bytes(&mut self, destination: u64, bytes: &[u8]) {
        if self.is_heap_address(destination) {
            self.write_bytes_heap(destination, bytes);
        } else {
            self.write_bytes_stack(destination, bytes);
        }
    }

    fn write_bytes_heap(&mut self, destination: u64, bytes: &[u8]) {
        for (i, byte) in bytes.iter().enumerate() {
            self.heap[destination as usize - Self::HEAP_OFFSET as usize + i] = *byte;
        }
    }

    fn write_bytes_stack(&mut self, destination: u64, bytes: &[u8]) {
        for (i, byte) in bytes.iter().enumerate() {
            self.stack[destination as usize - Self::STACK_OFFSET as usize + i] = *byte;
        }
    }
}
