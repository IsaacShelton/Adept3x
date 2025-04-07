mod assignment;
mod declaration;
mod kind;

pub use assignment::*;
pub use declaration::*;
pub use kind::*;
use source_files::Source;

#[derive(Clone, Debug)]
pub struct Stmt {
    pub kind: StmtKind,
    pub source: Source,
}

impl Stmt {
    pub fn new(kind: StmtKind, source: Source) -> Self {
        Self { kind, source }
    }
}
