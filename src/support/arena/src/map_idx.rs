use crate::{Idx, NewId};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct MapIdx<K> {
    pub(crate) raw: K,
}

impl<K: NewId, V> From<Idx<K, V>> for MapIdx<K> {
    fn from(value: Idx<K, V>) -> Self {
        Self { raw: value.raw }
    }
}
