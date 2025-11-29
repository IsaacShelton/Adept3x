use crate::{Pf, TaskStatus};
use serde::ser::SerializeMap;
use std::collections::HashMap;

#[derive(Default)]
pub struct Cache<'e, P: Pf> {
    kv: HashMap<<P as Pf>::Req<'e>, Option<TaskStatus<'e, P>>>,
}

impl<'e, P: Pf> Cache<'e, P> {
    pub fn get(&self, key: &P::Req<'e>) -> Option<&Option<TaskStatus<'e, P>>> {
        self.kv.get(key)
    }

    pub fn get_mut(&mut self, key: &P::Req<'e>) -> Option<&mut Option<TaskStatus<'e, P>>> {
        self.kv.get_mut(key)
    }

    pub fn insert(&mut self, key: P::Req<'e>, value: Option<TaskStatus<'e, P>>) {
        self.kv.insert(key, value);
    }

    pub fn entry<'c, 'k>(&'c mut self, key: &'k P::Req<'e>) -> CacheEntry<'c, 'k, 'e, P> {
        CacheEntry { cache: self, key }
    }
}

impl<'e, P: Pf> serde::Serialize for Cache<'e, P> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let map = serializer.serialize_map(Some(self.kv.len()))?;

        for (_k, _v) in self.kv.iter() {
            // map.serialize_entry(k, v)?;
            todo!("serialize cache")
        }

        map.end()
    }
}

pub struct CacheEntry<'c, 'k, 'e, P: Pf> {
    key: &'k P::Req<'e>,
    cache: &'c mut Cache<'e, P>,
}

impl<'c, 'k, 'e, P: Pf> CacheEntry<'c, 'k, 'e, P> {
    pub fn or_insert_with(
        self,
        f: impl FnOnce() -> Option<TaskStatus<'e, P>>,
    ) -> &'c mut Option<TaskStatus<'e, P>> {
        // Why does Rust not have a better way to do this? Entry API requires pre-cloning...
        if self.cache.kv.get(self.key).is_none() {
            self.cache.kv.insert(self.key.clone(), (f)());
        }

        self.cache.kv.get_mut(self.key).unwrap()
    }
}
