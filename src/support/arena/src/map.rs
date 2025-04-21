use crate::{Arena, Idx, NewId};
use core::marker::PhantomData;

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct ArenaMap<K: NewId, V> {
    arena: Arena<K, Option<V>>,
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
    pub fn contains_key(&self, key: K) -> bool {
        key.into_usize() < self.arena.len()
            && self.arena[Idx {
                raw: key,
                phantom: PhantomData,
            }]
            .is_some()
    }

    #[inline]
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
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
    pub fn get(&self, idx: K) -> Option<&V> {
        if idx.into_usize() >= self.arena.len() {
            return None;
        }

        self.arena[Idx {
            raw: idx,
            phantom: PhantomData,
        }]
        .as_ref()
    }

    #[inline]
    pub fn get_mut(&mut self, idx: K) -> Option<&mut V> {
        if idx.into_usize() >= self.arena.len() {
            return None;
        }

        self.arena[Idx {
            raw: idx,
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
