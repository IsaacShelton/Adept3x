use crate::ast::Type;

#[derive(Clone, Debug, Default)]
pub struct Params {
    pub required: Vec<Param>,
    pub is_cstyle_vararg: bool,
}

impl Params {
    pub fn normal(parameters: impl IntoIterator<Item = Param>) -> Self {
        Self {
            required: parameters.into_iter().collect(),
            is_cstyle_vararg: false,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Param {
    pub name: String,
    pub ast_type: Type,
}

impl Param {
    pub fn new(name: String, ast_type: Type) -> Self {
        Self { name, ast_type }
    }
}
