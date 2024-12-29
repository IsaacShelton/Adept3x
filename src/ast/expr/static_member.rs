use super::Call;
use crate::{ast::Type, source_files::Source};

#[derive(Clone, Debug)]
pub struct StaticMember {
    pub subject: Type,
    pub action: StaticMemberAction,
    pub source: Source,
}

#[derive(Clone, Debug)]
pub struct StaticMemberAction {
    pub kind: StaticMemberActionKind,
    pub source: Source,
}

#[derive(Clone, Debug)]
pub enum StaticMemberActionKind {
    Value(String),
    Call(Call),
}

impl StaticMemberActionKind {
    pub fn at(self, source: Source) -> StaticMemberAction {
        StaticMemberAction { kind: self, source }
    }
}
