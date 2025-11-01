use super::Memory;
use crate::{
    interpret::{
        InterpreterError, Value,
        value::{Tainted, ValueKind},
    },
    ir,
};
use ir::Literal;
use primitives::{FloatSize, IntegerBits, IntegerSign};

impl Memory {
    pub fn read<'env>(
        &mut self,
        from: u64,
        ir_type: &ir::Type,
    ) -> Result<Value<'env>, InterpreterError> {
        if self.is_reserved_address(from) {
            return Err(InterpreterError::SegfaultRead);
        }

        use IntegerBits::*;
        use IntegerSign::*;

        Ok(match ir_type {
            ir::Type::Ptr(_) => self.read_u64(from),
            ir::Type::Bool => self.read_u1(from),
            ir::Type::I(Bits8, Signed) => self.read_s8(from),
            ir::Type::I(Bits16, Signed) => self.read_s16(from),
            ir::Type::I(Bits32, Signed) => self.read_s32(from),
            ir::Type::I(Bits64, Signed) => self.read_s64(from),
            ir::Type::I(Bits8, Unsigned) => self.read_u8(from),
            ir::Type::I(Bits16, Unsigned) => self.read_u16(from),
            ir::Type::I(Bits32, Unsigned) => self.read_u32(from),
            ir::Type::I(Bits64, Unsigned) => self.read_u64(from),
            ir::Type::F(FloatSize::Bits32) => self.read_f32(from),
            ir::Type::F(FloatSize::Bits64) => self.read_f64(from),
            ir::Type::Void => Value::new_untainted(ValueKind::Literal(Literal::Void)),
            ir::Type::Union(_) => todo!("interpreter read union"),
            ir::Type::Struct(_) => todo!("interpreter read structure"),
            ir::Type::AnonymousComposite(_) => todo!("interpreter read anonymous composite"),
            ir::Type::FuncPtr => self.read_u64(from),
            ir::Type::FixedArray(_) => todo!("interpreter read fixed array"),
            ir::Type::Vector(_) => todo!("interpreter read vector"),
            ir::Type::Complex(_) => todo!("interpreter read complex number"),
            ir::Type::Atomic(inner) => self.read(from, inner)?,
            ir::Type::IncompleteArray(_) => self.read_u64(from),
        })
    }

    pub fn read_bytes(&self, from: u64, count: usize) -> (&[u8], Option<Tainted>) {
        if self.is_heap_address(from) {
            let start = (from - Self::HEAP_OFFSET) as usize;

            let tainted = self
                .heap_tainted_by_comptime_sizeof
                .iter()
                .skip(start)
                .take(count)
                .any(|b| b)
                .then_some(Tainted::ByCompilationHostSizeof);
            (&self.heap[start..start + count], tainted)
        } else {
            let start = (from - Self::STACK_OFFSET) as usize;

            let tainted = self
                .stack_tainted_by_comptime_sizeof
                .iter()
                .skip(start)
                .take(count)
                .any(|b| b)
                .then_some(Tainted::ByCompilationHostSizeof);
            (&self.stack[start..start + count], tainted)
        }
    }

    pub fn read_u1<'a>(&self, from: u64) -> Value<'a> {
        let (bytes, tainted) = self.read_bytes(from, 1);
        Value::new(ValueKind::Literal(Literal::Boolean(bytes[0] != 0)), tainted)
    }

    pub fn read_u8<'a>(&self, from: u64) -> Value<'a> {
        let (bytes, tainted) = self.read_bytes(from, 1);

        Value::new(
            ValueKind::Literal(Literal::Unsigned8(u8::from_le_bytes(
                bytes.try_into().unwrap(),
            ))),
            tainted,
        )
    }

    pub fn read_u16<'a>(&self, from: u64) -> Value<'a> {
        let (bytes, tainted) = self.read_bytes(from, 2);
        Value::new(
            ValueKind::Literal(Literal::Unsigned16(u16::from_le_bytes(
                bytes.try_into().unwrap(),
            ))),
            tainted,
        )
    }

    pub fn read_u32<'a>(&self, from: u64) -> Value<'a> {
        let (bytes, tainted) = self.read_bytes(from, 4);
        Value::new(
            ValueKind::Literal(Literal::Unsigned32(u32::from_le_bytes(
                bytes.try_into().unwrap(),
            ))),
            tainted,
        )
    }

    pub fn read_u64<'a>(&self, from: u64) -> Value<'a> {
        let (bytes, tainted) = self.read_bytes(from, 8);
        Value::new(
            ValueKind::Literal(Literal::Unsigned64(u64::from_le_bytes(
                bytes.try_into().unwrap(),
            ))),
            tainted,
        )
    }

    pub fn read_s8<'a>(&self, from: u64) -> Value<'a> {
        let (bytes, tainted) = self.read_bytes(from, 1);
        Value::new(
            ValueKind::Literal(Literal::Signed8(i8::from_le_bytes(
                bytes.try_into().unwrap(),
            ))),
            tainted,
        )
    }

    pub fn read_s16<'a>(&self, from: u64) -> Value<'a> {
        let (bytes, tainted) = self.read_bytes(from, 2);
        Value::new(
            ValueKind::Literal(Literal::Signed16(i16::from_le_bytes(
                bytes.try_into().unwrap(),
            ))),
            tainted,
        )
    }

    pub fn read_s32<'a>(&self, from: u64) -> Value<'a> {
        let (bytes, tainted) = self.read_bytes(from, 4);
        Value::new(
            ValueKind::Literal(Literal::Signed32(i32::from_le_bytes(
                bytes.try_into().unwrap(),
            ))),
            tainted,
        )
    }

    pub fn read_s64<'a>(&self, from: u64) -> Value<'a> {
        let (bytes, tainted) = self.read_bytes(from, 8);
        Value::new(
            ValueKind::Literal(Literal::Signed64(i64::from_le_bytes(
                bytes.try_into().unwrap(),
            ))),
            tainted,
        )
    }

    pub fn read_f32<'a>(&self, from: u64) -> Value<'a> {
        let (bytes, tainted) = self.read_bytes(from, 4);
        Value::new(
            ValueKind::Literal(Literal::Float32(f32::from_le_bytes(
                bytes.try_into().unwrap(),
            ))),
            tainted,
        )
    }

    pub fn read_f64<'a>(&self, from: u64) -> Value<'a> {
        let (bytes, tainted) = self.read_bytes(from, 8);
        Value::new(
            ValueKind::Literal(Literal::Float64(f64::from_le_bytes(
                bytes.try_into().unwrap(),
            ))),
            tainted,
        )
    }
}
