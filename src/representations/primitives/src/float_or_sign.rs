use super::IntegerSign;

#[derive(Copy, Clone, Debug)]
pub enum FloatOrSign {
    Integer(IntegerSign),
    Float,
}
