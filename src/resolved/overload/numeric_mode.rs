use crate::{
    ast::{CInteger, IntegerBits},
    ir::IntegerSign,
    resolved,
};

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum NumericMode {
    Integer(IntegerSign),
    LooseIndeterminateSignInteger(CInteger),
    CheckOverflow(IntegerBits, IntegerSign),
    Float,
}

impl NumericMode {
    pub fn try_new(unified_type: &resolved::Type) -> Option<NumericMode> {
        match &unified_type.kind {
            resolved::TypeKind::Integer(_, sign) => Some(NumericMode::Integer(*sign)),
            resolved::TypeKind::CInteger(c_integer, sign) => {
                if let Some(sign) = sign {
                    Some(NumericMode::Integer(*sign))
                } else {
                    Some(NumericMode::LooseIndeterminateSignInteger(*c_integer))
                }
            }
            resolved::TypeKind::Floating(_) => Some(NumericMode::Float),
            _ => None,
        }
    }
}
