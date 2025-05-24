use super::Type;
use attributes::Privacy;
use indexmap::IndexMap;
use source_files::Source;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TypeHead<'env> {
    pub name: &'env str,
    pub arity: usize,
}

impl<'env> TypeHead<'env> {
    pub fn new(name: &'env str, arity: usize) -> Self {
        Self { name, arity }
    }
}

#[derive(Clone, Debug)]
pub enum TypeBody<'env> {
    Struct(StructBody<'env>),
    Enum(),
    Alias(),
    Trait(),
}

pub type TypeParams = ast::TypeParams;

#[derive(Clone, Debug)]
pub struct StructBody<'env> {
    pub fields: IndexMap<&'env str, Field<'env>>,
    pub is_packed: bool,
    pub params: TypeParams,
    pub source: Source,
}

#[derive(Copy, Clone, Debug)]
pub struct Field<'env> {
    pub ty: &'env Type<'env>,
    pub privacy: Privacy,
    pub source: Source,
}
