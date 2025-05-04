use beef::lean::Cow as LeanCow;
use std::collections::HashMap;

#[derive(Debug)]
pub enum Artifact<'outside> {
    Void,
    String(String),
    Str(&'outside str),
    Identifiers(HashMap<LeanCow<'outside, str>, ()>),
}

impl<'outside> Artifact<'outside> {
    pub fn unwrap_string(&self) -> &str {
        if let Self::String(string) = self {
            return string;
        }

        panic!("Expected execution artifact to be string");
    }
}
