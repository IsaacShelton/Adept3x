use super::Type;
use source_files::Source;

#[derive(Clone, Debug)]
pub struct Given {
    pub name: Option<(String, Source)>,
    pub ty: Type,
}
