use super::Memory;
use crate::{
    interpreter::{error::InterpreterError, value::Value},
    ir::{self, Literal},
};

impl Memory {
    pub fn read(
        &mut self,
        from: u64,
        ir_type: &ir::Type,
        ir_module: &ir::Module,
    ) -> Result<Value, InterpreterError> {
        if self.is_reserved_address(from) {
            return Err(InterpreterError::SegfaultRead);
        }

        Ok(match ir_type {
            ir::Type::Pointer(_) => self.read_u64(from),
            ir::Type::Boolean => self.read_u1(from),
            ir::Type::S8 => self.read_s8(from),
            ir::Type::S16 => self.read_s16(from),
            ir::Type::S32 => self.read_s32(from),
            ir::Type::S64 => self.read_s64(from),
            ir::Type::U8 => self.read_u8(from),
            ir::Type::U16 => self.read_u16(from),
            ir::Type::U32 => self.read_u32(from),
            ir::Type::U64 => self.read_u64(from),
            ir::Type::F32 => self.read_f32(from),
            ir::Type::F64 => self.read_f64(from),
            ir::Type::Void => Value::Literal(Literal::Void),
            ir::Type::Structure(_) => todo!("interpreter read structure"),
            ir::Type::AnonymousComposite(_) => todo!("interpreter read anonymous composite"),
            ir::Type::FunctionPointer => self.read_u64(from),
            ir::Type::FixedArray(_) => todo!("interpreter read fixed array"),
            ir::Type::Vector(_) => todo!("interpreter read vector"),
            ir::Type::Complex(_) => todo!("interpreter read complex number"),
            ir::Type::Atomic(inner) => self.read(from, inner, ir_module)?,
            ir::Type::IncompleteArray(_) => self.read_u64(from),
        })
    }

    fn read_bytes(&mut self, from: u64, count: usize) -> &[u8] {
        if self.is_heap_address(from) {
            let start = (from - Self::HEAP_OFFSET) as usize;
            &self.heap[start..start + count]
        } else {
            let start = (from - Self::STACK_OFFSET) as usize;
            &self.stack[start..start + count]
        }
    }

    fn read_u1(&mut self, from: u64) -> Value {
        let bytes = self.read_bytes(from, 1);
        Value::Literal(Literal::Boolean(bytes[0] != 0))
    }

    pub fn read_u8(&mut self, from: u64) -> Value {
        let bytes = self.read_bytes(from, 1);
        Value::Literal(Literal::Unsigned8(u8::from_le_bytes(
            bytes.try_into().unwrap(),
        )))
    }

    fn read_u16(&mut self, from: u64) -> Value {
        let bytes = self.read_bytes(from, 2);
        Value::Literal(Literal::Unsigned16(u16::from_le_bytes(
            bytes.try_into().unwrap(),
        )))
    }

    fn read_u32(&mut self, from: u64) -> Value {
        let bytes = self.read_bytes(from, 4);
        Value::Literal(Literal::Unsigned32(u32::from_le_bytes(
            bytes.try_into().unwrap(),
        )))
    }

    fn read_u64(&mut self, from: u64) -> Value {
        let bytes = self.read_bytes(from, 8);
        Value::Literal(Literal::Unsigned64(u64::from_le_bytes(
            bytes.try_into().unwrap(),
        )))
    }

    fn read_s8(&mut self, from: u64) -> Value {
        let bytes = self.read_bytes(from, 1);
        Value::Literal(Literal::Signed8(i8::from_le_bytes(
            bytes.try_into().unwrap(),
        )))
    }

    fn read_s16(&mut self, from: u64) -> Value {
        let bytes = self.read_bytes(from, 2);
        Value::Literal(Literal::Signed16(i16::from_le_bytes(
            bytes.try_into().unwrap(),
        )))
    }

    fn read_s32(&mut self, from: u64) -> Value {
        let bytes = self.read_bytes(from, 4);
        Value::Literal(Literal::Signed32(i32::from_le_bytes(
            bytes.try_into().unwrap(),
        )))
    }

    fn read_s64(&mut self, from: u64) -> Value {
        let bytes = self.read_bytes(from, 8);
        Value::Literal(Literal::Signed64(i64::from_le_bytes(
            bytes.try_into().unwrap(),
        )))
    }

    fn read_f32(&mut self, from: u64) -> Value {
        let bytes = self.read_bytes(from, 4);
        Value::Literal(Literal::Float32(f32::from_le_bytes(
            bytes.try_into().unwrap(),
        )))
    }

    fn read_f64(&mut self, from: u64) -> Value {
        let bytes = self.read_bytes(from, 8);
        Value::Literal(Literal::Float64(f64::from_le_bytes(
            bytes.try_into().unwrap(),
        )))
    }
}