use crate::asg::{Params, Type};

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct FuncPtr {
    pub params: Params,
    pub return_type: Box<Type>,
}
