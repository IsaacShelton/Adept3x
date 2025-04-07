use crate::{Param, Type};

#[derive(Clone, Debug)]
pub struct FuncPtr {
    pub parameters: Vec<Param>,
    pub return_type: Box<Type>,
    pub is_cstyle_variadic: bool,
}
