use super::Type;
use attributes::Privacy;
use indexmap::IndexMap;
use source_files::Source;

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
