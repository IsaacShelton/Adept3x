use crate::repr::DeclScope;
use asg::Asg;
use ast_workspace::AstWorkspace;
use beef::lean::Cow as LeanCow;
use std::collections::HashMap;

#[derive(Debug)]
pub enum Artifact<'env> {
    Void,
    String(String),
    Str(&'env str),
    Identifiers(HashMap<LeanCow<'env, str>, ()>),
    Asg(Asg<'env>),
    AstWorkspace(&'env AstWorkspace<'env>),
    EstimatedDeclScope(DeclScope),
}

macro_rules! artifact_unwrap_fn {
    ($self:expr, $variant:ident) => {
        if let Self::$variant(value) = $self {
            return value;
        } else {
            panic!(concat!("Expected artifact to be ", stringify!($variant)));
        }
    };
}

impl<'env> Artifact<'env> {
    pub fn unwrap_string(&self) -> &str {
        artifact_unwrap_fn!(self, String)
    }

    pub fn unwrap_ast_workspace(&self) -> &'env AstWorkspace<'env> {
        artifact_unwrap_fn!(self, AstWorkspace)
    }

    pub fn unwrap_estimated_decl_scope(&self) -> &DeclScope {
        artifact_unwrap_fn!(self, EstimatedDeclScope)
    }
}
