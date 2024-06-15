use indexmap::{Equivalent, IndexMap};
use std::hash::BuildHasher;

pub fn try_insert_into_index_map<
    K: Equivalent<K> + std::hash::Hash + std::cmp::Eq,
    V,
    S: BuildHasher,
    E,
>(
    index_map: &mut IndexMap<K, V, S>,
    key: K,
    value: V,
    or_else: impl Fn(K) -> E,
) -> Result<(), E> {
    // Unfortantely there isn't an API provided to do this without having
    // to do the lookup twice or cloning the key, so we will prefer the double lookup.
    if index_map.contains_key(&key) {
        Err(or_else(key))
    } else {
        assert!(index_map.insert(key, value).is_none());
        Ok(())
    }
}
