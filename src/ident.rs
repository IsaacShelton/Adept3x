use indexmap::Equivalent;
use std::hash::Hash;

pub struct Ident {
    pub name: Box<str>,
}

pub struct IdentRef<'a> {
    pub name: &'a str,
}

impl<T: Into<Box<str>>> From<T> for Ident {
    fn from(name: T) -> Self {
        Self { name: name.into() }
    }
}

impl<'a> From<&'a str> for IdentRef<'a> {
    fn from(name: &'a str) -> Self {
        Self { name }
    }
}

impl<'a> Equivalent<Ident> for IdentRef<'a> {
    fn equivalent(&self, key: &Ident) -> bool {
        *self.name == *key.name
    }
}

impl Hash for Ident {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.name.hash(state)
    }
}

impl<'a> Hash for IdentRef<'a> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.name.hash(state)
    }
}
