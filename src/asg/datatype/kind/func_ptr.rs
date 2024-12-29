use crate::asg::{Parameter, Type};

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct FuncPtr {
    pub parameters: Vec<Parameter>,
    pub return_type: Box<Type>,
    pub is_cstyle_variadic: bool,
}
