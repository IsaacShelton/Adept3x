use crate::{
    Id, Idx, IdxSpan,
    idx_span::IdxSpanIter,
    iter::{IntoIter, Iter, IterMut},
    values::{Values, ValuesMut},
};
use alloc::vec::Vec;
use core::{
    fmt,
    hash::{Hash, Hasher},
    marker::PhantomData,
    ops::{Index, IndexMut},
};

/// A index-based arena.
///
/// [`Arena`] provides a mechanism to allocate objects and refer to them by a
/// strongly-typed index ([`Idx<K, V>`]). The index not only represents the position
/// in the underlying vector but also leverages the type system to prevent accidental misuse
/// across different arenas.
pub struct Arena<K: Id, V> {
    data: Vec<V>,
    phantom: PhantomData<(K, V)>,
}

impl<K: Id, V> Arena<K, V> {
    /// Creates a new empty arena.
    ///
    /// # Examples
    ///
    /// ```
    /// # use arena::Arena;
    /// let arena: Arena<u32, i32> = Arena::new();
    /// assert!(arena.is_empty());
    /// ```
    #[inline]
    pub const fn new() -> Self {
        Self {
            data: Vec::new(),
            phantom: PhantomData,
        }
    }

    #[inline]
    pub(crate) unsafe fn from_vec(data: Vec<V>) -> Self {
        Self {
            data,
            phantom: PhantomData,
        }
    }

    /// Creates a new arena with the specified capacity.
    ///
    /// # Examples
    ///
    /// ```
    /// # use arena::Arena;
    /// let arena: Arena<u32, i32> = Arena::with_capacity(10);
    /// assert!(arena.is_empty());
    /// assert!(arena.capacity() >= 10);
    /// ```
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            data: Vec::with_capacity(capacity),
            phantom: PhantomData,
        }
    }

    /// Returns the number of elements stored in the arena.
    ///
    /// # Examples
    ///
    /// ```
    /// # use arena::Arena;
    /// let mut arena = Arena::<u32, _>::new();
    /// assert_eq!(arena.len(), 0);
    ///
    /// arena.alloc("foo");
    /// assert_eq!(arena.len(), 1);
    ///
    /// arena.alloc("bar");
    /// assert_eq!(arena.len(), 2);
    ///
    /// arena.alloc("baz");
    /// assert_eq!(arena.len(), 3);
    /// ```
    #[inline]
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Returns the capacity of the arena.
    ///
    /// # Examples
    ///
    /// ```
    /// use arena::Arena;
    ///
    /// let arena: Arena<u32, String> = Arena::with_capacity(10);
    /// assert!(arena.capacity() >= 10);
    /// ```
    #[inline]
    pub fn capacity(&self) -> usize {
        self.data.capacity()
    }

    /// Returns `true` if the arena contains no elements.
    ///
    /// # Examples
    ///
    /// ```
    /// # use arena::Arena;
    /// let mut arena = Arena::<u32, _>::new();
    /// assert!(arena.is_empty());
    ///
    /// arena.alloc(0.9);
    /// assert!(!arena.is_empty());
    /// ```
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Allocates an element in the arena and returns its index.
    ///
    /// # Panics
    ///
    /// Panics if the arena is full (i.e. if the number of elements exceeds `I::MAX`).
    /// If you hnadle this case, use [`Arena::try_alloc`] instead.
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
    #[inline]
    pub fn alloc(&mut self, value: V) -> Idx<K, V> {
        self.try_alloc(value).expect("arena is full")
    }

    /// Fallible version of [`Arena::alloc`].
    ///
    /// This method returns `None` if the arena is full.
    #[inline]
    pub fn try_alloc(&mut self, value: V) -> Option<Idx<K, V>> {
        if self.data.len() < K::MAX {
            let id = K::from_usize(self.data.len());
            self.data.push(value);

            Some(Idx {
                raw: id,
                phantom: PhantomData,
            })
        } else {
            None
        }
    }

    /// Allocates multiple elements in the arena and returns the index span covering them.
    ///
    /// # Panics
    ///
    /// Panics if the arena cannot allocate all elements (i.e. if the arena becomes full).
    ///
    /// # Examples
    ///
    /// ```
    /// use arena::{Arena, IdxSpan};
    ///
    /// let mut arena: Arena<u32, i32> = Arena::new();
    /// let span: IdxSpan<u32, i32> = arena.alloc_many([10, 20, 30]);
    /// assert_eq!(&arena[span], &[10, 20, 30]);
    /// ```
    #[inline]
    pub fn alloc_many(&mut self, values: impl IntoIterator<Item = V>) -> IdxSpan<K, V> {
        self.try_alloc_many(values).expect("arena is full")
    }

    /// Fallible version of [`Arena::alloc_many`].
    ///
    /// This method returns `None` if the arena becomes full.
    #[inline]
    pub fn try_alloc_many(&mut self, values: impl IntoIterator<Item = V>) -> Option<IdxSpan<K, V>> {
        let start = K::from_usize(self.data.len());
        let mut len = self.data.len();
        for value in values {
            if len >= K::MAX {
                return None;
            }
            self.data.push(value);
            len += 1;
        }
        let end = K::from_usize(len);
        assert!(start <= end);
        Some(IdxSpan::new(start..end))
    }

    /// Returns a iterator over the elements and their indices in the arena.
    ///
    /// # Examples
    ///
    /// ```
    /// # use arena::Arena;
    /// let mut arena = Arena::<u32, _>::new();
    ///
    /// let idx1 = arena.alloc(20);
    /// let idx2 = arena.alloc(40);
    /// let idx3 = arena.alloc(60);
    ///
    /// let mut iter = arena.iter();
    /// assert_eq!(iter.next(), Some((idx1, &20)));
    /// assert_eq!(iter.next(), Some((idx2, &40)));
    /// assert_eq!(iter.next(), Some((idx3, &60)));
    /// assert_eq!(iter.next(), None);
    /// ```
    #[inline]
    pub fn iter(&self) -> Iter<'_, K, V> {
        Iter {
            iter: self.data.iter().enumerate(),
            phantom: PhantomData,
        }
    }

    /// Returns a mutable iterator over the elements and their indices in the arena.
    ///
    /// # Examples
    ///
    /// ```
    /// # use arena::Arena;
    /// let mut arena = Arena::<u32, _>::new();
    /// let idx1 = arena.alloc(20);
    ///
    /// assert_eq!(arena[idx1], 20);
    ///
    /// let mut iterator = arena.iter_mut();
    /// *iterator.next().unwrap().1 = 10;
    /// drop(iterator);
    ///
    /// assert_eq!(arena[idx1], 10);
    /// ```
    #[inline]
    pub fn iter_mut(&mut self) -> IterMut<'_, K, V> {
        IterMut {
            iter: self.data.iter_mut().enumerate(),
            phantom: PhantomData,
        }
    }

    #[inline]
    pub fn keys(&self) -> impl Iterator<Item = Idx<K, V>> {
        IdxSpanIter {
            next: K::from_usize(0),
            end: K::from_usize(self.data.len()),
            phantom: PhantomData,
        }
    }

    /// Returns an iterator over the values in the arena.
    ///
    /// # Examples
    ///
    /// ```
    /// # use arena::Arena;
    /// let mut arena = Arena::<u32, _>::new();
    /// arena.alloc_many([1, 2, 3]);
    ///
    /// let mut iter = arena.values();
    /// assert_eq!(iter.next(), Some(&1));
    /// assert_eq!(iter.next(), Some(&2));
    /// assert_eq!(iter.next(), Some(&3));
    /// assert_eq!(iter.next(), None);
    /// ```
    #[inline]
    pub fn values(&self) -> Values<'_, K, V> {
        Values {
            iter: self.data.iter(),
            phantom: PhantomData,
        }
    }

    /// Returns a mutable iterator over the values in the arena.
    ///
    /// # Examples
    ///
    /// ```
    /// # use arena::Arena;
    /// let mut arena = Arena::<u32, _>::new();
    /// arena.alloc_many([1, 2, 3]);
    ///
    /// let mut iter = arena.values_mut();
    /// *iter.next().unwrap() = 10;
    /// *iter.next().unwrap() = 20;
    /// *iter.next().unwrap() = 30;
    /// assert_eq!(iter.next(), None);
    ///
    /// let mut values = arena.values().cloned().collect::<Vec<_>>();
    /// assert_eq!(values, vec![10, 20, 30]);
    /// ```
    #[inline]
    pub fn values_mut(&mut self) -> ValuesMut<'_, K, V> {
        ValuesMut {
            iter: self.data.iter_mut(),
            phantom: PhantomData,
        }
    }

    /// Shrinks the capacity of the arena to fit the number of elements.
    ///
    /// # Examples
    ///
    /// ```
    /// # use arena::Arena;
    /// let mut arena = Arena::<u32, _>::with_capacity(10);
    /// arena.alloc_many(&[1, 2, 3]);
    /// assert!(arena.capacity() >= 10);
    ///
    /// arena.shrink_to_fit();
    /// assert!(arena.capacity() >= 3);
    /// ```
    #[inline]
    pub fn shrink_to_fit(&mut self) {
        self.data.shrink_to_fit();
    }
}

impl<K: Id, V: Default> Arena<K, V> {
    #[inline]
    pub fn grow_to_fit(&mut self, index: K) -> Idx<K, V> {
        let index = index.into_usize();

        if self.len() < index + 1 {
            let needs = index + 1 - self.len();
            self.alloc_many(core::iter::repeat_with(|| V::default()).take(needs));
        }

        Idx {
            raw: K::from_usize(index),
            phantom: PhantomData,
        }
    }
}

impl<K: Id, V> Default for Arena<K, V> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<K: Id, V> Index<Idx<K, V>> for Arena<K, V> {
    type Output = V;

    #[inline]
    fn index(&self, idx: Idx<K, V>) -> &Self::Output {
        &self.data[idx.raw.into_usize()]
    }
}

impl<K: Id, V> Index<IdxSpan<K, V>> for Arena<K, V> {
    type Output = [V];

    #[inline]
    fn index(&self, span: IdxSpan<K, V>) -> &Self::Output {
        &self.data[span.start.into_usize()..span.end.into_usize()]
    }
}

impl<K: Id, V> IndexMut<Idx<K, V>> for Arena<K, V> {
    #[inline]
    fn index_mut(&mut self, idx: Idx<K, V>) -> &mut Self::Output {
        &mut self.data[idx.raw.into_usize()]
    }
}

impl<K: Id, V> IndexMut<IdxSpan<K, V>> for Arena<K, V> {
    #[inline]
    fn index_mut(&mut self, span: IdxSpan<K, V>) -> &mut Self::Output {
        &mut self.data[span.start.into_usize()..span.end.into_usize()]
    }
}

impl<K: Id, V: Clone> Clone for Arena<K, V> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            data: self.data.clone(),
            phantom: PhantomData,
        }
    }
}

impl<K: Id, V: fmt::Debug> fmt::Debug for Arena<K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Arena")
            .field("len", &self.len())
            .field("data", &self.data)
            .finish()
    }
}

impl<K: Id, V: PartialEq> PartialEq for Arena<K, V> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.data == other.data
    }
}

impl<K: Id, V: Eq> Eq for Arena<K, V> {}

impl<K: Id, V: Hash> Hash for Arena<K, V> {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.data.hash(state)
    }
}

impl<'a, K: Id, V> IntoIterator for &'a Arena<K, V> {
    type Item = (Idx<K, V>, &'a V);
    type IntoIter = Iter<'a, K, V>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<K: Id, V> IntoIterator for Arena<K, V> {
    type Item = (Idx<K, V>, V);
    type IntoIter = IntoIter<K, V>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        IntoIter {
            iter: self.data.into_iter().enumerate(),
            phantom: PhantomData,
        }
    }
}
