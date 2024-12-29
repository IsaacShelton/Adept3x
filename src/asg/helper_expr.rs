use super::TypedExpr;
use crate::ast::Privacy;

#[derive(Clone, Debug)]
pub struct HelperExprDecl {
    pub value: TypedExpr,
    pub privacy: Privacy,
}
