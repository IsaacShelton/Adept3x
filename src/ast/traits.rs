use super::Privacy;
use crate::source_files::Source;

#[derive(Clone, Debug)]
pub struct Trait {
    pub name: String,
    pub source: Source,
    pub privacy: Privacy,
}
