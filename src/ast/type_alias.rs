use super::{Privacy, Type};
use crate::source_files::Source;

#[derive(Clone, Debug)]
pub struct TypeAlias {
    pub value: Type,
    pub source: Source,
    pub privacy: Privacy,
}
