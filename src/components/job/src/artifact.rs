use crate::repr::{DeclScope, TypeHead};
use asg::Asg;
use ast_workspace::TypeDeclRef;
use derive_more::From;

#[derive(Debug, From)]
pub enum Artifact<'env> {
    Void(()),
    Asg(&'env Asg<'env>),
    DeclScope(&'env DeclScope),
    TypeHead(&'env TypeHead<'env>),
    TypeHeads(&'env [&'env TypeHead<'env>]),
    OptionTypeDeclRef(Option<TypeDeclRef>),
    OptionAsgType(Option<&'env asg::Type>),
}

impl_unwrap_from_artifact!(Void, ());
impl_unwrap_from_artifact!(Asg, &'env asg::Asg<'env>);
impl_unwrap_from_artifact!(DeclScope, &'env DeclScope);
impl_unwrap_from_artifact!(TypeHead, &'env TypeHead<'env>);
impl_unwrap_from_artifact!(TypeHeads, &'env [&'env TypeHead<'env>]);
impl_unwrap_from_artifact!(OptionTypeDeclRef, Option<TypeDeclRef>);
impl_unwrap_from_artifact!(OptionAsgType, Option<&'env asg::Type>);
