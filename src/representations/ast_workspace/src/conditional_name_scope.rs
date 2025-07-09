use crate::NameScopeRef;
use ast::Expr;

#[derive(Clone, Debug)]
pub struct ConditionalNameScope {
    pub conditions: Vec<(Expr, NameScopeRef)>,
    pub otherwise: Option<NameScopeRef>,
}
