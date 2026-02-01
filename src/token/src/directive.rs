use std::fmt::Display;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Directive {
    Standard(&'static str),
    Unknown(Box<str>),
}

impl Directive {
    pub fn new(directive: &'static str) -> Self {
        Self::Standard(directive)
    }

    pub fn unknown(unknown: Box<str>) -> Self {
        Self::Unknown(unknown)
    }

    pub fn len_with_prefix(&self) -> usize {
        1 + match self {
            Directive::Standard(s) => s.len(),
            Directive::Unknown(s) => s.len(),
        }
    }
}

impl AsRef<str> for Directive {
    fn as_ref(&self) -> &str {
        match self {
            Directive::Standard(s) => s,
            Directive::Unknown(s) => s,
        }
    }
}

impl Display for Directive {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "@{}", self.as_ref())
    }
}
