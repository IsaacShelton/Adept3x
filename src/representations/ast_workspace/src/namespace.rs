use crate::NameScopeRef;
use attributes::Privacy;

#[derive(Clone, Debug)]
pub struct Namespace {
    pub name: String,
    pub names: NameScopeRef,
    pub privacy: Privacy,
}
