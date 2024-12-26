use crate::ast::Type;

#[derive(Clone, Debug, Default)]
pub struct Parameters {
    pub required: Vec<Parameter>,
    pub is_cstyle_vararg: bool,
}

impl Parameters {
    pub fn normal(parameters: impl IntoIterator<Item = Parameter>) -> Self {
        Self {
            required: parameters.into_iter().collect(),
            is_cstyle_vararg: false,
        }
    }
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
