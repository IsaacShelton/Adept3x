use crate::line_column::Location;

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
    pub location: Location,
}

impl Annotation {
    pub fn new(kind: AnnotationKind, location: Location) -> Self {
        Self { kind, location }
    }
}

impl ToString for AnnotationKind {
    fn to_string(&self) -> String {
        match self {
            Self::Foreign => "foreign",
            Self::ThreadLocal => "thread_local",
            Self::Packed => "packed",
            Self::Pod => "pod",
            Self::AbideAbi => "abide_abi",
        }
        .into()
    }
}
