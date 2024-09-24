use crate::name::Name;
use std::fmt::Debug;

pub struct Named<T> {
    pub name: Name,
    pub value: T,
}

impl<T: Clone> Clone for Named<T> {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            value: self.value.clone(),
        }
    }
}

impl<T: Debug> Debug for Named<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Named")
            .field("name", &self.name)
            .field("value", &self.value)
            .finish()
    }
}
