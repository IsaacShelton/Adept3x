use crate::{Arena, Id, Idx, NewId};
use core::marker::PhantomData;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ArenaMap<K: NewId, V> {
    arena: Arena<K, Option<V>>,
}

impl<K: NewId, V> Default for ArenaMap<K, V> {
    fn default() -> Self {
        Self {
            arena: Default::default(),
        }
    }
}

impl<K: NewId, V> ArenaMap<K, V> {
    #[inline]
    pub fn new() -> Self {
        Self {
            arena: Arena::new(),
        }
    }

    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            arena: Arena::with_capacity(capacity),
        }
    }

    #[inline]
    pub fn contains_key(&self, key: impl IntoRaw<K>) -> bool {
        let key = key.into_raw();
        key.into_usize() < self.arena.len()
            && self.arena[Idx {
                raw: key,
                phantom: PhantomData,
            }]
            .is_some()
    }

    #[inline]
    pub fn insert(&mut self, key: impl IntoRaw<K>, value: V) -> Option<V> {
        let key = key.into_raw();
        let idx = self.arena.grow_to_fit(key);
        core::mem::replace(&mut self.arena[idx], Some(value))
    }

    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = (K, &V)> {
        self.arena.iter().filter_map(|(idx, maybe_value)| {
            maybe_value.as_ref().map(|value| (idx.into_raw(), value))
        })
    }

    #[inline]
    pub fn get(&self, idx: impl IntoRaw<K>) -> Option<&V> {
        let key = idx.into_raw();
        if key.into_usize() >= self.arena.len() {
            return None;
        }

        self.arena[Idx {
            raw: key,
            phantom: PhantomData,
        }]
        .as_ref()
    }

    #[inline]
    pub fn get_mut(&mut self, idx: impl IntoRaw<K>) -> Option<&mut V> {
        let key = idx.into_raw();
        if key.into_usize() >= self.arena.len() {
            return None;
        }

        self.arena[Idx {
            raw: key,
            phantom: PhantomData,
        }]
        .as_mut()
    }
}

impl<'a, K: NewId, V> IntoIterator for &'a ArenaMap<K, V> {
    type Item = (Idx<K, Option<V>>, &'a Option<V>);
    type IntoIter = <&'a Arena<K, Option<V>> as IntoIterator>::IntoIter;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.arena.iter()
    }
}

impl<K: NewId, V> IntoIterator for ArenaMap<K, V> {
    type Item = (Idx<K, Option<V>>, Option<V>);
    type IntoIter = <Arena<K, Option<V>> as IntoIterator>::IntoIter;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.arena.into_iter()
    }
}

pub trait IntoRaw<K: Id> {
    fn into_raw(self) -> K;
}

impl<K: Id> IntoRaw<K> for K {
    fn into_raw(self) -> K {
        self
    }
}

impl<K: Id, V> IntoRaw<K> for Idx<K, V> {
    fn into_raw(self) -> K {
        self.into_raw()
    }
}
