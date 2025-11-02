use super::Type;
use derive_more::{From, IsVariant, Unwrap};
use num_bigint::BigInt;
use primitives::{IntegerBits, IntegerConstant, IntegerSign};
use std::ffi::CStr;
use std_ext::SmallVec8;

#[derive(Copy, Clone, Debug, From)]
pub enum Value<'env> {
    Literal(Literal<'env>),
    Reference(ValueReference),
}

#[derive(Copy, Clone, Debug, Unwrap, IsVariant)]
pub enum Literal<'env> {
    Void,
    Boolean(bool),
    Integer(IntegerImmediate),
    Float32(f32),
    Float64(f64),
    NullTerminatedString(&'env CStr),
    Zeroed(&'env Type<'env>),
}

impl<'env> Literal<'env> {
    pub fn new_integer(value: impl Into<IntegerConstant>, bits: IntegerBits) -> Option<Self> {
        IntegerImmediate::new(value.into(), bits).map(Self::Integer)
    }

    pub fn unwrap_signed(&self) -> i64 {
        match self {
            Literal::Integer(immediate) => immediate.value().unwrap_signed(),
            Literal::Zeroed(_) => 0,
            _ => panic!("ir::Literal::unwrap_signed"),
        }
    }

    pub fn unwrap_unsigned(&self) -> u64 {
        match self {
            Literal::Integer(immediate) => immediate.value().unwrap_unsigned(),
            Literal::Zeroed(_) => 0,
            _ => panic!("ir::Literal::unwrap_unsigned"),
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct ValueReference {
    pub basicblock_id: usize,
    pub instruction_id: usize,
}

#[derive(Copy, Clone, Debug)]
pub struct IntegerImmediate {
    bits: IntegerBits,
    value: IntegerConstant,
}

impl IntegerImmediate {
    pub fn new(value: IntegerConstant, bits: IntegerBits) -> Option<Self> {
        if value.fits_in(bits) {
            Some(Self { bits, value })
        } else {
            None
        }
    }

    pub fn new_with_bits_and_sign(
        value: &BigInt,
        sign: IntegerSign,
        bits: IntegerBits,
    ) -> Option<Self> {
        let value = match sign {
            IntegerSign::Signed => value.try_into().map(IntegerConstant::Signed).ok()?,
            IntegerSign::Unsigned => value.try_into().map(IntegerConstant::Unsigned).ok()?,
        };

        Self::new(value, bits)
    }

    pub fn value(&self) -> IntegerConstant {
        self.value
    }

    pub fn bits(&self) -> IntegerBits {
        self.bits
    }

    pub fn mask(&self) -> u64 {
        self.bits.mask()
    }

    pub fn to_le_bytes(&self) -> SmallVec8<u8> {
        self.value()
            .raw_data()
            .to_le_bytes()
            .into_iter()
            .take(self.bits.bytes().bytes() as usize)
            .collect()
    }
}
