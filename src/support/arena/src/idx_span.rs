use crate::{Id, Idx, simple_type_name::simple_type_name};
use core::{
    fmt,
    hash::{Hash, Hasher},
    iter::FusedIterator,
    marker::PhantomData,
    ops::Range,
};

/// A span of indices within an `Arena`.
///
/// This type represents a contiguous range of allocated indices in an arena.
///
/// # Examples
///
/// ```
/// use arena::{Arena, IdxSpan};
///
/// let mut arena: Arena<u32, i32> = Arena::new();
/// let span: IdxSpan<u32, i32> = arena.alloc_many(1..=4);
/// assert_eq!(span.len(), 4);
/// assert!(!span.is_empty());
/// assert_eq!(&arena[span], &[1, 2, 3, 4]);
/// ```
pub struct IdxSpan<K: Id, V> {
    pub(crate) start: K,
    pub(crate) end: K,
    pub(crate) phantom: PhantomData<fn() -> V>,
}

impl<K: Id, V> IdxSpan<K, V> {
    /// Creates a new [`IdxSpan`] from the given range of raw indices.
    #[inline]
    pub const fn new(range: Range<K>) -> Self {
        Self {
            start: range.start,
            end: range.end,
            phantom: PhantomData,
        }
    }

    /// Returns the starting raw index.
    #[inline]
    pub const fn start(&self) -> K {
        self.start
    }

    /// Returns the ending raw index.
    #[inline]
    pub const fn end(&self) -> K {
        self.end
    }

    /// Returns the number of indices in the span.
    #[inline]
    pub fn len(&self) -> usize {
        self.end.into_usize() - self.start.into_usize()
    }

    /// Returns true if the span is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }

    /// Iterates through the range
    #[inline]
    pub fn iter(&self) -> IdxSpanIter<K, V> {
        IdxSpanIter {
            next: self.start,
            end: self.end,
            phantom: PhantomData,
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct IdxSpanIter<K: Id, V> {
    pub(crate) next: K,
    pub(crate) end: K,
    pub(crate) phantom: PhantomData<fn() -> V>,
}

impl<K: Id, V> FusedIterator for IdxSpanIter<K, V> {}

impl<K: Id, V> Iterator for IdxSpanIter<K, V> {
    type Item = Idx<K, V>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.next.into_usize() < self.end.into_usize() {
            let key = self.next;
            self.next = self.next.successor();

            Some(Idx {
                raw: key,
                phantom: PhantomData,
            })
        } else {
            None
        }
    }
}

impl<K: Id, V> IntoIterator for IdxSpan<K, V> {
    type Item = Idx<K, V>;
    type IntoIter = IdxSpanIter<K, V>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<K: Id, V> Clone for IdxSpan<K, V> {
    #[inline]
    fn clone(&self) -> Self {
        *self
    }
}
impl<K: Id, V> Copy for IdxSpan<K, V> {}

impl<K: Id + fmt::Debug, V> fmt::Debug for IdxSpan<K, V> {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        let key_name = simple_type_name::<K>();
        let value_name = simple_type_name::<V>();
        write!(
            fmt,
            "IdxSpan::<{}, {}>({:?}..{:?})",
            key_name, value_name, self.start, self.end
        )
    }
}

impl<K: Id, V> PartialEq for IdxSpan<K, V> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.start == other.start && self.end == other.end
    }
}
impl<K: Id, V> Eq for IdxSpan<K, V> {}

impl<K: Id + Hash, V> Hash for IdxSpan<K, V> {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.start.hash(state);
        self.end.hash(state);
    }
}
