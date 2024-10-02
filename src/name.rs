use std::fmt::Display;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Name {
    pub namespace: Box<str>,
    pub basename: Box<str>,
}

impl Name {
    pub fn new(namespace: Option<impl Into<String>>, basename: impl Into<String>) -> Self {
        Self {
            namespace: namespace
                .map(|namespace| namespace.into())
                .unwrap_or_default()
                .into_boxed_str(),
            basename: basename.into().into_boxed_str(),
        }
    }

    pub fn plain(basename: impl Into<String>) -> Self {
        Self {
            namespace: "".into(),
            basename: basename.into().into_boxed_str(),
        }
    }

    pub fn into_plain(self) -> Option<String> {
        if self.namespace.is_empty() {
            Some(self.basename.to_string())
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

    pub fn fullname(&self) -> String {
        if self.namespace.is_empty() {
            self.basename.clone().to_string()
        } else {
            format!("{}/{}", self.namespace, self.basename)
        }
    }
}

impl Display for Name {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.fullname())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ResolvedName {
    Remote(Box<str>),
    Project(Box<str>),
}

impl ResolvedName {
    pub fn new(name: &Name) -> Self {
        Self::Project(name.fullname().into_boxed_str())
    }

    pub fn new_remote(name: &Name) -> Self {
        Self::Remote(name.fullname().into_boxed_str())
    }

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
            ResolvedName::Project(name) => write!(f, "{}", name),
        }
    }
}
