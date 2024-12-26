use std::{cmp::Eq, collections::HashMap, hash::Hash};

pub trait HashMapExt<K: Hash + Eq, V> {
    fn try_insert<E, FnOrElse: Fn(&K) -> E>(
        &mut self,
        key: &K,
        value: V,
        or_else: FnOrElse,
    ) -> Result<(), E>;

    fn get_or_insert_with(&mut self, key: &K, make_val: impl Fn() -> V) -> &mut V;
}

impl<K: Hash + Eq + Clone, V> HashMapExt<K, V> for HashMap<K, V> {
    fn try_insert<E, FnOrElse: Fn(&K) -> E>(
        &mut self,
        key: &K,
        value: V,
        or_else: FnOrElse,
    ) -> Result<(), E> {
        // Unfortantely there isn't an API provided to do this without having
        // to do the lookup twice or cloning the key, so we will prefer the double lookup.
        if self.contains_key(key) {
            Err(or_else(key))
        } else {
            assert!(self.insert(key.clone(), value).is_none());
            Ok(())
        }
    }

    fn get_or_insert_with(&mut self, key: &K, make_val: impl Fn() -> V) -> &mut V {
        // Due to overly strict borrow checking in the stable rust borrow checker, this is the best we can do.

        if !self.contains_key(key) {
            self.insert(key.clone(), make_val());
        }

        self.get_mut(key).unwrap()
    }
}
