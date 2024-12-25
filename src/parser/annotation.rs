use crate::{ast, source_files::Source};
use std::fmt::Display;

#[derive(Clone, Debug)]
pub struct Annotation {
    pub kind: AnnotationKind,
    pub source: Source,
}

impl Annotation {
    pub fn new(kind: AnnotationKind, source: Source) -> Self {
        Self { kind, source }
    }
}

#[derive(Clone, Debug)]
pub enum AnnotationKind {
    Foreign,
    ThreadLocal,
    Packed,
    AbideAbi,
    Public,
    Template,
    Given(Given),
}

#[derive(Clone, Debug)]
pub struct Given {
    pub name: Option<String>,
    pub ty: ast::Type,
}

impl AnnotationKind {
    pub fn at(self, source: Source) -> Annotation {
        Annotation { kind: self, source }
    }
}

impl Display for AnnotationKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Foreign => "foreign",
            Self::ThreadLocal => "thread_local",
            Self::Packed => "packed",
            Self::AbideAbi => "abide_abi",
            Self::Public => "public",
            Self::Template => "template",
            Self::Given(_) => "given",
        })
    }
}
