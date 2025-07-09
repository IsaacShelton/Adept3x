use crate::{Expr, NamespaceItems};

#[derive(Clone, Debug)]
pub struct ConditionalCompilation {
    pub conditions: Vec<(Expr, NamespaceItems)>,
    pub otherwise: Option<NamespaceItems>,
}
