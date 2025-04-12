use crate::{IdxSpan, NewId};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct MapIdxSpan<K> {
    pub(crate) start: K,
    pub(crate) end: K,
}

impl<K: NewId, V> From<IdxSpan<K, V>> for MapIdxSpan<K> {
    fn from(value: IdxSpan<K, V>) -> Self {
        Self {
            start: value.start,
            end: value.end,
        }
    }
}
