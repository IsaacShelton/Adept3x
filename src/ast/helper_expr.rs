use super::Expr;
use crate::source_files::Source;

#[derive(Debug, Clone)]
pub struct HelperExpr {
    pub value: Expr,
    pub source: Source,
    pub is_file_local_only: bool,
}
