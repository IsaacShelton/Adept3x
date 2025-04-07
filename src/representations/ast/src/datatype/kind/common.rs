use super::TypeKind;
use primitives::{CInteger, FloatSize, IntegerBits, IntegerSign};

// Common Basic Types
impl TypeKind {
    pub fn i8() -> Self {
        Self::Integer(IntegerBits::Bits8, IntegerSign::Signed)
    }

    pub fn u8() -> Self {
        Self::Integer(IntegerBits::Bits8, IntegerSign::Unsigned)
    }

    pub fn i16() -> Self {
        Self::Integer(IntegerBits::Bits16, IntegerSign::Signed)
    }

    pub fn u16() -> Self {
        Self::Integer(IntegerBits::Bits16, IntegerSign::Unsigned)
    }

    pub fn i32() -> Self {
        Self::Integer(IntegerBits::Bits32, IntegerSign::Signed)
    }

    pub fn u32() -> Self {
        Self::Integer(IntegerBits::Bits32, IntegerSign::Unsigned)
    }

    pub fn i64() -> Self {
        Self::Integer(IntegerBits::Bits64, IntegerSign::Signed)
    }

    pub fn u64() -> Self {
        Self::Integer(IntegerBits::Bits64, IntegerSign::Unsigned)
    }

    pub fn f32() -> Self {
        Self::Floating(FloatSize::Bits32)
    }

    pub fn f64() -> Self {
        Self::Floating(FloatSize::Bits64)
    }

    pub fn char() -> Self {
        Self::CInteger(CInteger::Char, None)
    }

    pub fn schar() -> Self {
        Self::CInteger(CInteger::Char, Some(IntegerSign::Signed))
    }

    pub fn uchar() -> Self {
        Self::CInteger(CInteger::Char, Some(IntegerSign::Unsigned))
    }

    pub fn short() -> Self {
        Self::CInteger(CInteger::Short, Some(IntegerSign::Signed))
    }

    pub fn ushort() -> Self {
        Self::CInteger(CInteger::Short, Some(IntegerSign::Unsigned))
    }

    pub fn int() -> Self {
        Self::CInteger(CInteger::Int, Some(IntegerSign::Signed))
    }

    pub fn uint() -> Self {
        Self::CInteger(CInteger::Int, Some(IntegerSign::Unsigned))
    }

    pub fn long() -> Self {
        Self::CInteger(CInteger::Long, Some(IntegerSign::Signed))
    }

    pub fn ulong() -> Self {
        Self::CInteger(CInteger::Long, Some(IntegerSign::Unsigned))
    }

    pub fn longlong() -> Self {
        Self::CInteger(CInteger::LongLong, Some(IntegerSign::Signed))
    }

    pub fn ulonglong() -> Self {
        Self::CInteger(CInteger::LongLong, Some(IntegerSign::Unsigned))
    }

    pub fn isize() -> Self {
        Self::SizeInteger(IntegerSign::Signed)
    }

    pub fn usize() -> Self {
        Self::SizeInteger(IntegerSign::Unsigned)
    }
}
