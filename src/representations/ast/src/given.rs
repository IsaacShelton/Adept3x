use super::Type;
use source_files::Sourced;

#[derive(Clone, Debug)]
pub struct Given {
    pub name: Option<Sourced<String>>,
    pub ty: Type,
}
