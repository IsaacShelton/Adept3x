use crate::ir::IntegerSign;

#[derive(Copy, Clone, Debug)]
pub enum FloatOrSign {
    Integer(IntegerSign),
    Float,
}
