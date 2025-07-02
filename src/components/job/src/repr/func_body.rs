use super::Variables;

#[derive(Clone, Debug)]
pub struct FuncBody<'env> {
    // pub stmts: Vec<Stmt>,
    pub variables: Variables<'env>,
}
