use std::marker::PhantomData;

#[derive(Clone, Debug)]
pub struct FuncBody<'env> {
    _phantom: PhantomData<&'env ()>,
    // pub stmts: Vec<Stmt>,
    // pub vars: VariableStorage,
}
