use super::Type;
use crate::source_files::Source;

#[derive(Clone, Debug)]
pub struct Given {
    pub name: Option<(String, Source)>,
    pub ty: Type,
}
