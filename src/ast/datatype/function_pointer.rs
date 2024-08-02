use super::Type;
use crate::ast::Parameter;

#[derive(Clone, Debug)]
pub struct FunctionPointer {
    pub parameters: Vec<Parameter>,
    pub return_type: Box<Type>,
    pub is_cstyle_variadic: bool,
}
