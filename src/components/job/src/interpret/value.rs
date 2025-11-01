use crate::ir;
use derive_more::{IsVariant, Unwrap};

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Tainted {
    ByCompilationHostSizeof,
}

#[derive(Clone, Debug)]
pub struct Value<'a> {
    pub kind: ValueKind<'a>,
    pub tainted: Option<Tainted>,
}

impl<'a> Value<'a> {
    pub fn new(kind: ValueKind<'a>, tainted: Option<Tainted>) -> Self {
        Self { kind, tainted }
    }

    pub fn new_tainted(kind: ValueKind<'a>, tainted: Tainted) -> Self {
        Self {
            kind,
            tainted: Some(tainted),
        }
    }

    pub fn new_untainted(kind: ValueKind<'a>) -> Self {
        Self {
            kind,
            tainted: None,
        }
    }
}

#[derive(Clone, Debug, Unwrap, IsVariant)]
pub enum ValueKind<'env> {
    Undefined,
    Literal(ir::Literal<'env>),
    StructLiteral(StructLiteral<'env>),
}

impl<'a> ValueKind<'a> {
    pub fn untainted(self) -> Value<'a> {
        Value {
            kind: self,
            tainted: None,
        }
    }

    pub fn tainted(self, tainted: Tainted) -> Value<'a> {
        Value {
            kind: self,
            tainted: Some(tainted),
        }
    }
}

#[derive(Clone, Debug)]
pub struct StructLiteral<'env> {
    pub values: Vec<Value<'env>>,
    pub fields: &'env [ir::Field<'env>],
}

impl<'env> Value<'env> {
    pub fn as_u64(&self) -> Option<u64> {
        match &self.kind {
            ValueKind::Literal(literal) => match literal {
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
