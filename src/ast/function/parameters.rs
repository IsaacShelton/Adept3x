use crate::ast::Type;

#[derive(Clone, Debug, Default)]
pub struct Parameters {
    pub required: Vec<Parameter>,
    pub is_cstyle_vararg: bool,
}

#[derive(Clone, Debug)]
pub struct Parameter {
    pub name: String,
    pub ast_type: Type,
}

impl Parameter {
    pub fn new(name: String, ast_type: Type) -> Self {
        Self { name, ast_type }
    }
}
