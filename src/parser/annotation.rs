use crate::ast::Source;
use std::fmt::Display;

#[derive(Clone, Debug)]
pub enum AnnotationKind {
    Foreign,
    ThreadLocal,
    Packed,
    Pod,
    AbideAbi,
}

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

impl Display for AnnotationKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Foreign => "foreign",
            Self::ThreadLocal => "thread_local",
            Self::Packed => "packed",
            Self::Pod => "pod",
            Self::AbideAbi => "abide_abi",
        })
    }
}
