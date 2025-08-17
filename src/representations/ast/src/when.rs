use crate::{Expr, NamespaceItems};

#[derive(Clone, Debug)]
pub struct When {
    pub conditions: Vec<(Expr, NamespaceItems)>,
    pub otherwise: Option<NamespaceItems>,
}
