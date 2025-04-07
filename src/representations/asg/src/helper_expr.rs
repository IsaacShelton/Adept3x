use attributes::Privacy;

#[derive(Clone, Debug)]
pub struct HelperExprDecl {
    pub value: ast::Expr,
    pub privacy: Privacy,
}
