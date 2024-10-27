use super::FloatOrSign;
use crate::{ast::CInteger, ir::IntegerSign, target::Target};

#[derive(Copy, Clone, Debug)]
pub enum FloatOrSignLax {
    Integer(IntegerSign),
    IndeterminateInteger(CInteger),
    Float,
}

impl FloatOrSignLax {
    pub fn or_default_for(&self, target: &Target) -> FloatOrSign {
        match self {
            FloatOrSignLax::Integer(sign) => FloatOrSign::Integer(*sign),
            FloatOrSignLax::IndeterminateInteger(c_integer) => {
                FloatOrSign::Integer(target.default_c_integer_sign(*c_integer))
            }
            FloatOrSignLax::Float => FloatOrSign::Float,
        }
    }
}
