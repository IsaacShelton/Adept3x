use crate::{Arena, Id, Idx, IdxSpan, idx_span::IdxSpanIter};
use append_only_vec::AppendOnlyVec;
use core::{
    marker::PhantomData,
    ops::{Index, IndexMut},
};

/// A lock-free index-based arena.
///
/// [`LockFreeArena`] provides a mechanism to allocate objects and refer to them by a
/// strongly-typed index ([`Idx<K, V>`]). The index not only represents the position
/// in the underlying vector but also leverages the type system to prevent accidental misuse
/// across different arenas.
#[derive(Debug)]
pub struct LockFreeArena<K: Id, V> {
    data: AppendOnlyVec<V>,
    phantom: PhantomData<(K, V)>,
}

unsafe impl<K: Id + Send, V: Send> Send for LockFreeArena<K, V> {}
unsafe impl<K: Id + Send + Sync, V: Send + Sync> Sync for LockFreeArena<K, V> {}

impl<K: Id, V> LockFreeArena<K, V> {
    /// Creates a new empty arena.
    ///
    /// # Examples
    ///
    /// ```
    /// # use arena::LockFreeArena;
    /// let arena: LockFreeArena<u32, i32> = LockFreeArena::new();
    /// assert_eq!(arena.len(), 0);
    /// ```
    #[inline]
    pub const fn new() -> Self {
        Self {
            data: AppendOnlyVec::new(),
            phantom: PhantomData,
        }
    }

    pub fn into_arena(self) -> Arena<K, V> {
        unsafe { Arena::from_vec(self.data.into_vec()) }
    }

    /// Returns the number of elements stored in the arena.
    ///
    /// # Examples
    ///
    /// ```
    /// # use arena::LockFreeArena;
    /// let mut arena = LockFreeArena::<u32, _>::new();
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

    /// Allocates an element in the arena and returns its index.
    ///
    /// # Panics
    ///
    /// Panics if the arena is full (i.e. if the number of elements exceeds `I::MAX`).
    /// If you hnadle this case, use [`LockFreeArena::try_alloc`] instead.
    ///
    /// # Examples
    ///
    /// ```
    /// use arena::{LockFreeArena, Idx};
    ///
    /// let mut arena: LockFreeArena<u32, &str> = LockFreeArena::new();
    /// let idx: Idx<u32, &str> = arena.alloc("hello");
    /// assert_eq!(arena[idx], "hello");
    /// ```
    #[inline]
    pub fn alloc(&self, value: V) -> Idx<K, V> {
        self.try_alloc(value).expect("arena is full")
    }

    /// Fallible version of [`LockFreeArena::alloc`].
    ///
    /// This method returns `None` if the arena is full.
    #[inline]
    pub fn try_alloc(&self, value: V) -> Option<Idx<K, V>> {
        let index = self.data.push(value);

        (index < K::MAX).then(|| Idx {
            raw: K::from_usize(index),
            phantom: PhantomData,
        })
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
            self.data.push_mut(value);
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
    /// # use arena::LockFreeArena;
    /// let mut arena = LockFreeArena::<u32, _>::new();
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
    pub fn iter(&self) -> impl Iterator<Item = (Idx<K, V>, &V)> {
        self.data.iter().enumerate().map(|(index, v)| {
            (
                Idx {
                    raw: K::from_usize(index),
                    phantom: PhantomData,
                },
                v,
            )
        })
    }

    #[inline]
    pub fn keys(&self) -> impl Iterator<Item = Idx<K, V>> {
        IdxSpanIter {
            next: K::from_usize(0),
            end: K::from_usize(self.data.len()),
            phantom: PhantomData,
        }
    }

    #[inline]
    pub fn get_span(&self, span: IdxSpan<K, V>) -> impl Iterator<Item = &V> {
        (span.start.into_usize()..span.end.into_usize()).map(|index| &self.data[index])
    }
}

impl<K: Id, V> Default for LockFreeArena<K, V> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<K: Id, V> Index<Idx<K, V>> for LockFreeArena<K, V> {
    type Output = V;

    #[inline]
    fn index(&self, idx: Idx<K, V>) -> &Self::Output {
        &self.data[idx.raw.into_usize()]
    }
}

impl<K: Id, V> IndexMut<Idx<K, V>> for LockFreeArena<K, V> {
    #[inline]
    fn index_mut(&mut self, idx: Idx<K, V>) -> &mut Self::Output {
        &mut self.data[idx.raw.into_usize()]
    }
}
