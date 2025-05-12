use crate::repr::StaticScope;
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
    StaticScope(StaticScope),
}

impl<'env> Artifact<'env> {
    pub fn unwrap_string(&self) -> &str {
        if let Self::String(string) = self {
            return string;
        }

        panic!("Expected execution artifact to be string");
    }

    pub fn unwrap_ast_workspace(&self) -> &'env AstWorkspace<'env> {
        if let Self::AstWorkspace(ast_workspace) = self {
            return ast_workspace;
        }

        panic!("Expected execution artifact to be AstWorkspace");
    }

    pub fn unwrap_static_scope(&self) -> &StaticScope {
        if let Self::StaticScope(static_scope) = self {
            return static_scope;
        }

        panic!("Expected execution artifact to be StaticScope");
    }
}
