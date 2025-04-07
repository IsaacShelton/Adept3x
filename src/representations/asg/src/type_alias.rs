use super::{HumanName, Type, TypeParams};
use source_files::Source;

#[derive(Clone, Debug)]
pub struct TypeAlias {
    pub human_name: HumanName,
    pub source: Source,
    pub params: TypeParams,
    pub becomes: Type,
}
