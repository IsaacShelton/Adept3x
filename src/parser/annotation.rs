use crate::line_column::Location;

#[derive(Clone, Debug)]
pub enum AnnotationKind {
    Foreign,
    ThreadLocal,
    Packed,
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
            AnnotationKind::Foreign => "foreign",
            AnnotationKind::ThreadLocal => "thread_local",
            AnnotationKind::Packed => "packed",
        }
        .into()
    }
}
