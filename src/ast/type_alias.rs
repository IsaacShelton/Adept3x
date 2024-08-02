use super::Type;
use crate::source_files::Source;

#[derive(Clone, Debug)]
pub struct TypeAlias {
    pub value: Type,
    pub source: Source,
}
