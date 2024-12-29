use super::{Func, Privacy, Type};
use crate::source_files::Source;

#[derive(Clone, Debug)]
pub struct Impl {
    pub name: Option<String>,
    pub target: Type,
    pub source: Source,
    pub privacy: Privacy,
    pub body: Vec<Func>,
}
