use crate::ast::{self, Privacy};

#[derive(Clone, Debug)]
pub struct HelperExprDecl {
    pub value: ast::Expr,
    pub privacy: Privacy,
}
