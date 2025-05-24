use super::{Type, TypeParams};
use attributes::Privacy;
use source_files::Source;

#[derive(Clone, Debug)]
pub struct TypeAliasBody<'env> {
    pub target: &'env Type<'env>,
    pub params: TypeParams,
    pub privacy: Privacy,
    pub source: Source,
}
