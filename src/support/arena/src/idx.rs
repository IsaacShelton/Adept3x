use crate::{Id, MapIdx, NewId, simple_type_name::simple_type_name};
use core::{
    fmt,
    fmt::Formatter,
    hash::{Hash, Hasher},
    marker::PhantomData,
};

/// A typed index for referencing elements in an [`Arena`].
///
/// The [`Idx<K, V>`] type wraps an underlying id of type `K` and carries a phantom type `V`
/// to ensure type safety when indexing into an arena.
///
/// # Examples
///
/// ```
/// use arena::{Arena, Idx};
///
/// let mut arena: Arena<u32, &str> = Arena::new();
/// let idx: Idx<u32, &str> = arena.alloc("hello");
/// assert_eq!(arena[idx], "hello");
/// ```
pub struct Idx<K: Id, V> {
    pub(crate) raw: K,
    pub(crate) phantom: PhantomData<fn() -> V>,
}

impl<K: Id, V> Idx<K, V> {
    /// Consumes the index and returns its underlying raw value.
    ///
    /// # Examples
    ///
    /// ```
    /// use arena::{Arena, Idx};
    ///
    /// let mut arena: Arena<u32, i32> = Arena::new();
    /// let idx = arena.alloc(10);
    /// let raw = idx.into_raw();
    /// // raw is a u32 representing the index inside the arena.
    /// assert_eq!(raw, 0u32);
    /// ```
    #[inline]
    pub const fn into_raw(self) -> K {
        self.raw
    }

    #[inline]
    pub const fn reinterpet(self) -> MapIdx<K>
    where
        K: NewId,
    {
        MapIdx { raw: self.raw }
    }
}

impl<K: Id, V> Clone for Idx<K, V> {
    #[inline]
    fn clone(&self) -> Self {
        *self
    }
}

impl<K: Id, V> Copy for Idx<K, V> {}

impl<K: Id + fmt::Debug, V> fmt::Debug for Idx<K, V> {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> fmt::Result {
        let key_name = simple_type_name::<K>();
        let value_name = simple_type_name::<V>();
        write!(fmt, "Idx::<{}, {}>({:?})", key_name, value_name, self.raw)
    }
}

impl<K: Id, V> PartialEq for Idx<K, V> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.raw == other.raw
    }
}

impl<K: Id, V> Eq for Idx<K, V> {}

impl<K: Id, V> Ord for Idx<K, V> {
    #[inline]
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.raw.cmp(&other.raw)
    }
}

impl<K: Id, V> PartialOrd for Idx<K, V> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<K: Id + Hash, V> Hash for Idx<K, V> {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.raw.hash(state)
    }
}
