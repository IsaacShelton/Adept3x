use crate::resolved::{Parameter, Type};

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct FunctionPointer {
    pub parameters: Vec<Parameter>,
    pub return_type: Box<Type>,
    pub is_cstyle_variadic: bool,
}
