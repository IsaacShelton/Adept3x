use crate::{Pf, TaskStatus};
use serde::ser::SerializeMap;
use std::{collections::HashMap, io::Write, path::Path};

const HEADER: &[u8] =
    b"This file is a local cache and *not* sharable. It should be ignored for version control purposes.\n";
const COMPILER_BUILT_AT: u64 = compile_time::unix!();
const HUMAN_READABLE: bool = true;

#[derive(Default)]
pub struct Cache<'e, P: Pf> {
    kv: Kv<'e, P>,
}

#[derive(Default)]
pub struct Kv<'e, P: Pf> {
    inner: HashMap<<P as Pf>::Req<'e>, Option<TaskStatus<'e, P>>>,
}

impl<'e, P: Pf> Cache<'e, P> {
    pub fn get(&self, key: &P::Req<'e>) -> Option<&Option<TaskStatus<'e, P>>> {
        self.kv.inner.get(key)
    }

    pub fn get_mut(&mut self, key: &P::Req<'e>) -> Option<&mut Option<TaskStatus<'e, P>>> {
        self.kv.inner.get_mut(key)
    }

    pub fn insert(&mut self, key: P::Req<'e>, value: Option<TaskStatus<'e, P>>) {
        self.kv.inner.insert(key, value);
    }

    pub fn entry<'c, 'k>(&'c mut self, key: &'k P::Req<'e>) -> CacheEntry<'c, 'k, 'e, P> {
        CacheEntry { cache: self, key }
    }

    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), ()> {
        let mut file = std::fs::File::create(path).map_err(|_| ())?;
        file.write(HEADER).map_err(|_| ())?;

        if HUMAN_READABLE {
            serde_json::to_writer_pretty(&mut file, self).map_err(|_| ())?;
        } else {
            bincode::serde::encode_into_std_write(self, &mut file, bincode::config::standard())
                .map_err(|_| ())?;
        }

        Ok(())
    }
}

impl<'e, P: Pf> serde::Serialize for Cache<'e, P> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut header = serializer.serialize_map(Some(3))?;
        header.serialize_entry("v", &format!("{:X}", COMPILER_BUILT_AT))?;
        header.serialize_entry("kv", &self.kv)?;
        header.end()
    }
}

impl<'e, P: Pf> serde::Serialize for Kv<'e, P> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let map = serializer.serialize_map(Some(0))?;
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
        if self.cache.kv.inner.get(self.key).is_none() {
            self.cache.kv.inner.insert(self.key.clone(), (f)());
        }

        self.cache.kv.inner.get_mut(self.key).unwrap()
    }
}
