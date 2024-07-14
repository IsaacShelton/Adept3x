use super::Memory;
use crate::{
    interpreter::{error::InterpreterError, value::Value},
    ir::{self, Literal},
};

impl Memory {
    pub fn read<'a>(
        &mut self,
        from: u64,
        ir_type: &ir::Type,
    ) -> Result<Value<'a>, InterpreterError> {
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
            ir::Type::Union(_) => todo!("interpreter read union"),
            ir::Type::Structure(_) => todo!("interpreter read structure"),
            ir::Type::AnonymousComposite(_) => todo!("interpreter read anonymous composite"),
            ir::Type::FunctionPointer => self.read_u64(from),
            ir::Type::FixedArray(_) => todo!("interpreter read fixed array"),
            ir::Type::Vector(_) => todo!("interpreter read vector"),
            ir::Type::Complex(_) => todo!("interpreter read complex number"),
            ir::Type::Atomic(inner) => self.read(from, inner)?,
            ir::Type::IncompleteArray(_) => self.read_u64(from),
        })
    }

    pub fn read_bytes(&self, from: u64, count: usize) -> &[u8] {
        if self.is_heap_address(from) {
            let start = (from - Self::HEAP_OFFSET) as usize;
            &self.heap[start..start + count]
        } else {
            let start = (from - Self::STACK_OFFSET) as usize;
            &self.stack[start..start + count]
        }
    }

    pub fn read_u1<'a>(&self, from: u64) -> Value<'a> {
        let bytes = self.read_bytes(from, 1);
        Value::Literal(Literal::Boolean(bytes[0] != 0))
    }

    pub fn read_u8<'a>(&self, from: u64) -> Value<'a> {
        let bytes = self.read_bytes(from, 1);
        Value::Literal(Literal::Unsigned8(u8::from_le_bytes(
            bytes.try_into().unwrap(),
        )))
    }

    pub fn read_u16<'a>(&self, from: u64) -> Value<'a> {
        let bytes = self.read_bytes(from, 2);
        Value::Literal(Literal::Unsigned16(u16::from_le_bytes(
            bytes.try_into().unwrap(),
        )))
    }

    pub fn read_u32<'a>(&self, from: u64) -> Value<'a> {
        let bytes = self.read_bytes(from, 4);
        Value::Literal(Literal::Unsigned32(u32::from_le_bytes(
            bytes.try_into().unwrap(),
        )))
    }

    pub fn read_u64<'a>(&self, from: u64) -> Value<'a> {
        let bytes = self.read_bytes(from, 8);
        Value::Literal(Literal::Unsigned64(u64::from_le_bytes(
            bytes.try_into().unwrap(),
        )))
    }

    pub fn read_s8<'a>(&self, from: u64) -> Value<'a> {
        let bytes = self.read_bytes(from, 1);
        Value::Literal(Literal::Signed8(i8::from_le_bytes(
            bytes.try_into().unwrap(),
        )))
    }

    pub fn read_s16<'a>(&self, from: u64) -> Value<'a> {
        let bytes = self.read_bytes(from, 2);
        Value::Literal(Literal::Signed16(i16::from_le_bytes(
            bytes.try_into().unwrap(),
        )))
    }

    pub fn read_s32<'a>(&self, from: u64) -> Value<'a> {
        let bytes = self.read_bytes(from, 4);
        Value::Literal(Literal::Signed32(i32::from_le_bytes(
            bytes.try_into().unwrap(),
        )))
    }

    pub fn read_s64<'a>(&self, from: u64) -> Value<'a> {
        let bytes = self.read_bytes(from, 8);
        Value::Literal(Literal::Signed64(i64::from_le_bytes(
            bytes.try_into().unwrap(),
        )))
    }

    pub fn read_f32<'a>(&self, from: u64) -> Value<'a> {
        let bytes = self.read_bytes(from, 4);
        Value::Literal(Literal::Float32(f32::from_le_bytes(
            bytes.try_into().unwrap(),
        )))
    }

    pub fn read_f64<'a>(&self, from: u64) -> Value<'a> {
        let bytes = self.read_bytes(from, 8);
        Value::Literal(Literal::Float64(f64::from_le_bytes(
            bytes.try_into().unwrap(),
        )))
    }
}
