use std::fmt::Display;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Name {
    pub namespace: String,
    pub basename: String,
}

impl Name {
    pub fn plain(basename: impl Into<String>) -> Self {
        Self {
            namespace: "".into(),
            basename: basename.into(),
        }
    }

    pub fn into_plain(self) -> Option<String> {
        if self.namespace.is_empty() {
            Some(self.basename)
        } else {
            None
        }
    }

    pub fn as_plain_str(&self) -> Option<&str> {
        if self.namespace.is_empty() {
            Some(&self.basename)
        } else {
            None
        }
    }
}

impl Display for Name {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", self.namespace, self.basename)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ResolvedName {
    Remote(Box<str>),
    Project(Box<str>),
}

impl ResolvedName {
    pub fn plain(&self) -> &str {
        match self {
            ResolvedName::Remote(name) => &**name,
            ResolvedName::Project(name) => &**name,
        }
    }
}

impl Display for ResolvedName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResolvedName::Remote(name) => write!(f, "<remote>/{}", name),
            ResolvedName::Project(name) => write!(f, "<project>/{}", name),
        }
    }
}
