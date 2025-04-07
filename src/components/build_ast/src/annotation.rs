use ast::Given;
use source_files::Source;
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
    Exposed,
    ThreadLocal,
    Packed,
    AbideAbi,
    Public,
    Private,
    Template,
    Using(Given),
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
            Self::Exposed => "exposed",
            Self::ThreadLocal => "thread_local",
            Self::Packed => "packed",
            Self::AbideAbi => "abide_abi",
            Self::Public => "public",
            Self::Private => "private",
            Self::Template => "template",
            Self::Using(_) => "using",
        })
    }
}
