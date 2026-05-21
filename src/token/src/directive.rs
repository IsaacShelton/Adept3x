use serde::{Deserialize, Serialize};
use std::fmt::Display;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Directive {
    Keyword(Box<str>),
    Standard(Box<str>),
    Unknown(Box<str>),
}

impl Directive {
    pub fn new(directive: Box<str>) -> Self {
        Self::Standard(directive)
    }

    pub fn unknown(unknown: Box<str>) -> Self {
        Self::Unknown(unknown)
    }

    pub fn len_with_prefix(&self) -> usize {
        match self {
            Directive::Keyword(s) => s.len(),
            Directive::Standard(s) => 1 + s.len(),
            Directive::Unknown(s) => 1 + s.len(),
        }
    }
}

impl Display for Directive {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Directive::Keyword(s) => write!(f, "{s}"),
            Directive::Standard(s) => write!(f, "@{s}"),
            Directive::Unknown(s) => write!(f, "@{s}"),
        }
    }
}
