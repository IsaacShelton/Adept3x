use super::Type;
use crate::ast::Param;

#[derive(Clone, Debug)]
pub struct FuncPtr {
    pub parameters: Vec<Param>,
    pub return_type: Box<Type>,
    pub is_cstyle_variadic: bool,
}
