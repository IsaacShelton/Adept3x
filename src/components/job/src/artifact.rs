use crate::repr::{
    AmbiguousType, DeclScope, Field, FindTypeResult, FuncHead, Type, TypeArg, TypeBody, TypeHead,
};
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
    FindType(Result<Option<TypeDeclRef>, AmbiguousType>),
    TypeBody(&'env TypeBody<'env>),
    Field(Field<'env>),
    Type(&'env Type<'env>),
    TypeArg(&'env TypeArg<'env>),
    FuncHead(&'env FuncHead<'env>),
}

impl_unwrap_from_artifact!(Void, ());
impl_unwrap_from_artifact!(Asg, &'env asg::Asg<'env>);
impl_unwrap_from_artifact!(DeclScope, &'env DeclScope);
impl_unwrap_from_artifact!(TypeHead, &'env TypeHead<'env>);
impl_unwrap_from_artifact!(TypeHeads, &'env [&'env TypeHead<'env>]);
impl_unwrap_from_artifact!(OptionTypeDeclRef, Option<TypeDeclRef>);
impl_unwrap_from_artifact!(OptionAsgType, Option<&'env asg::Type>);
impl_unwrap_from_artifact!(FindType, FindTypeResult);
impl_unwrap_from_artifact!(TypeBody, &'env TypeBody<'env>);
impl_unwrap_from_artifact!(Field, Field<'env>);
impl_unwrap_from_artifact!(Type, &'env Type<'env>);
impl_unwrap_from_artifact!(TypeArg, &'env TypeArg<'env>);
impl_unwrap_from_artifact!(FuncHead, &'env FuncHead<'env>);
