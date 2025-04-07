use crate::{Call, Type};
use source_files::Source;

#[derive(Clone, Debug)]
pub struct StaticMemberValue {
    pub subject: Type,
    pub value: String,
    pub value_source: Source,
    pub source: Source,
}

#[derive(Clone, Debug)]
pub struct StaticMemberCall {
    pub subject: Type,
    pub call: Call,
    pub call_source: Source,
    pub source: Source,
}
