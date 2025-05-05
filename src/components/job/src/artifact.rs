use asg::Asg;
use ast_workspace::AstWorkspace;
use beef::lean::Cow as LeanCow;
use std::collections::HashMap;

#[derive(Debug)]
pub enum Artifact<'outside> {
    Void,
    String(String),
    Str(&'outside str),
    Identifiers(HashMap<LeanCow<'outside, str>, ()>),
    Asg(Asg<'outside>),
    AstWorkspace(&'outside AstWorkspace<'outside>),
}

impl<'outside> Artifact<'outside> {
    pub fn unwrap_string(&self) -> &str {
        if let Self::String(string) = self {
            return string;
        }

        panic!("Expected execution artifact to be string");
    }

    pub fn unwrap_ast_workspace(&self) -> &'outside AstWorkspace<'outside> {
        if let Self::AstWorkspace(ast_workspace) = self {
            return ast_workspace;
        }

        panic!("Expected execution artifact to be AstWorkspace");
    }
}
