use crate::ir;
use derive_more::{IsVariant, Unwrap};

#[derive(Clone, Debug, Unwrap, IsVariant)]
pub enum Value<'a> {
    Undefined,
    Literal(ir::Literal),
    StructLiteral(StructLiteral<'a>),
}

#[derive(Clone, Debug)]
pub struct StructLiteral<'a> {
    pub values: Vec<Value<'a>>,
    pub fields: &'a [ir::Field],
}

impl<'a> Value<'a> {
    pub fn as_u64(&self) -> Option<u64> {
        match self {
            Value::Literal(literal) => match literal {
                ir::Literal::Boolean(value) => Some((*value).into()),
                ir::Literal::Signed8(value) => (*value).try_into().ok(),
                ir::Literal::Signed16(value) => (*value).try_into().ok(),
                ir::Literal::Signed32(value) => (*value).try_into().ok(),
                ir::Literal::Signed64(value) => (*value).try_into().ok(),
                ir::Literal::Unsigned8(value) => Some((*value).into()),
                ir::Literal::Unsigned16(value) => Some((*value).into()),
                ir::Literal::Unsigned32(value) => Some((*value).into()),
                ir::Literal::Unsigned64(value) => Some(*value),
                ir::Literal::Zeroed(ty) if ty.is_integer_like() => Some(0),
                _ => None,
            },
            _ => None,
        }
    }
}
