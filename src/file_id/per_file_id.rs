use super::FileId;
use std::{borrow::Borrow, collections::HashMap};

#[derive(Clone, Debug)]
pub struct PerFileId<T> {
    backing: HashMap<FileId, T>,
}

impl<T> Default for PerFileId<T> {
    fn default() -> Self {
        Self {
            backing: Default::default(),
        }
    }
}

impl<T> PerFileId<T> {
    pub fn get_or_insert_with(&mut self, id: FileId, make_val: impl Fn() -> T) -> &mut T {
        self.backing.entry(id).or_insert_with(make_val)
    }

    pub fn get(&self, id: impl Borrow<FileId>) -> Option<&T> {
        self.backing.get(id.borrow())
    }

    pub fn get_mut(&mut self, id: impl Borrow<FileId>) -> Option<&mut T> {
        self.backing.get_mut(id.borrow())
    }
}
