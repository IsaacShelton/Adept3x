use super::Memory;
use crate::{
    interpret::{
        InterpreterError, Value,
        value::{Tainted, ValueKind},
    },
    ir,
};
use ir::Literal;
use primitives::{FloatSize, IntegerBits, IntegerConstant, IntegerSign};

impl Memory {
    pub fn read<'env>(
        &mut self,
        from: u64,
        ir_type: &ir::Type,
    ) -> Result<Value<'env>, InterpreterError> {
        if self.is_reserved_address(from) {
            return Err(InterpreterError::SegfaultRead);
        }

        Ok(match ir_type {
            ir::Type::Ptr(_) | ir::Type::FuncPtr | ir::Type::IncompleteArray(_) => {
                self.read_integer(from, IntegerSign::Unsigned, IntegerBits::Bits64)
            }
            ir::Type::Bool => self.read_u1(from),
            ir::Type::I(bits, sign) => self.read_integer(from, *sign, *bits),
            ir::Type::F(FloatSize::Bits32) => self.read_f32(from),
            ir::Type::F(FloatSize::Bits64) => self.read_f64(from),
            ir::Type::Void => Value::new_untainted(ValueKind::Literal(Literal::Void)),
            ir::Type::Union(_) => todo!("interpreter read union"),
            ir::Type::Struct(_) => todo!("interpreter read structure"),
            ir::Type::AnonymousComposite(_) => todo!("interpreter read anonymous composite"),
            ir::Type::FixedArray(_) => todo!("interpreter read fixed array"),
            ir::Type::Vector(_) => todo!("interpreter read vector"),
            ir::Type::Complex(_) => todo!("interpreter read complex number"),
            ir::Type::Atomic(inner) => self.read(from, inner)?,
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

    pub fn read_u1<'env>(&self, from: u64) -> Value<'env> {
        let (bytes, tainted) = self.read_bytes(from, 1);
        Value::new(ValueKind::Literal(Literal::Boolean(bytes[0] != 0)), tainted)
    }

    pub fn read_integer<'env>(
        &self,
        from: u64,
        sign: IntegerSign,
        bits: IntegerBits,
    ) -> Value<'env> {
        let (le_bytes, tainted) = self.read_bytes(from, bits.bytes().bytes() as usize);
        let constant = IntegerConstant::from_le(le_bytes, sign);
        Value::new(
            ValueKind::Literal(ir::Literal::new_integer(constant, bits).unwrap()),
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
