mod de;
mod entry;
mod kv;
mod ser;

use crate::{Pf, TaskStatus, log};
pub use entry::*;
pub use kv::*;
use serde_json::de::IoRead;
use std::{
    io::{Seek, Write},
    path::Path,
};

const HEADER: &[u8] =
    b"This file is a local cache and *not* sharable. It should be ignored for version control purposes.\n";
const HUMAN_READABLE: bool = true;

#[derive(Debug, Default)]
pub struct Cache<'e, P: Pf> {
    kv: Kv<'e, P>,
}

impl<'e, P: Pf> Cache<'e, P> {
    pub fn load(path: impl AsRef<Path>) -> Cache<'e, P> {
        match Self::try_load(path) {
            Ok(restored) => {
                log!("DISK - RESTORED FROM CACHE {:?}", &restored);
                restored
            }
            Err(_) => {
                log!("DISK - COULD NOT RESTORE FROM CACHE");
                Self::default()
            }
        }
    }

    pub fn try_load(path: impl AsRef<Path>) -> Result<Cache<'e, P>, ()> {
        let mut file = std::fs::File::open(path).map_err(|_| ())?;
        file.seek_relative(HEADER.len().try_into().unwrap())
            .map_err(|_| ())?;
        let io_read = IoRead::new(&mut file);
        let mut de = serde_json::Deserializer::new(io_read);
        let value = <Self as serde::de::Deserialize>::deserialize(&mut de).map_err(|_| ())?;
        de.end().map_err(|_| ())?;
        Ok(value)
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
}
