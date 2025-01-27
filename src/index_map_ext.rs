use indexmap::{Equivalent, IndexMap};
use std::{cmp::Eq, hash::Hash};

pub trait IndexMapExt<K: Equivalent<K> + Hash + Eq, V> {
    fn try_insert<E, FnOrElse: Fn(K) -> E>(
        &mut self,
        key: K,
        value: V,
        or_else: FnOrElse,
    ) -> Result<(), E>;

    fn get_or_insert_with(&mut self, key: K, make_val: impl Fn() -> V) -> &mut V;

    fn insert_or_panic(&mut self, key: K, value: V);

    fn has_items(&self) -> bool;
}

impl<K: Equivalent<K> + Hash + Eq, V> IndexMapExt<K, V> for IndexMap<K, V> {
    fn try_insert<E, FnOrElse: Fn(K) -> E>(
        &mut self,
        key: K,
        value: V,
        or_else: FnOrElse,
    ) -> Result<(), E> {
        // Unfortantely there isn't an API provided to do this without having
        // to do the lookup twice or cloning the key, so we will prefer the double lookup.
        if self.contains_key(&key) {
            Err(or_else(key))
        } else {
            assert!(self.insert(key, value).is_none());
            Ok(())
        }
    }

    fn get_or_insert_with(&mut self, key: K, make_val: impl Fn() -> V) -> &mut V {
        self.entry(key).or_insert_with(make_val)
    }

    fn insert_or_panic(&mut self, key: K, value: V) {
        assert!(self.insert(key, value).is_none());
    }

    fn has_items(&self) -> bool {
        !self.is_empty()
    }
}
