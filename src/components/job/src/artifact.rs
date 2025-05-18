use crate::repr::DeclScope;
use asg::Asg;
use derive_more::From;

#[derive(Debug, From)]
pub enum Artifact<'env> {
    Void(()),
    Asg(Asg<'env>),
    DeclScope(DeclScope),
}

impl_unwrap_from!(Void, ());
impl_unwrap_from!(Asg, asg::Asg<'env>);
impl_unwrap_from!(DeclScope, DeclScope);
