use super::Expr;
use crate::source_files::Source;

#[derive(Clone, Debug)]
pub struct Define {
    pub value: Expr,
    pub source: Source,
}
