#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TypeHead {
    pub name: String,
    pub arity: usize,
}

impl TypeHead {
    pub fn new(name: String, arity: usize) -> Self {
        Self { name, arity }
    }
}
