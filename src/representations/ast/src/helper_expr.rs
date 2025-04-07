use super::Expr;
use attributes::Privacy;
use source_files::Source;

#[derive(Debug, Clone)]
pub struct HelperExpr {
    pub name: String,
    pub value: Expr,
    pub source: Source,
    pub is_file_local_only: bool,
    pub privacy: Privacy,
}
