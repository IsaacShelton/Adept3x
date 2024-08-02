mod assignment;
mod declaration;
mod kind;

use crate::source_files::Source;
pub use assignment::*;
pub use declaration::*;
pub use kind::*;

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
