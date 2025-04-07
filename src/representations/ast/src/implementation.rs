use super::{Func, Type, TypeParams};
use attributes::Privacy;
use source_files::Source;

#[derive(Clone, Debug)]
pub struct Impl {
    pub name: Option<String>,
    pub params: TypeParams,
    pub target: Type,
    pub source: Source,
    pub privacy: Privacy,
    pub body: Vec<Func>,
}
