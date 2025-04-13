use crate::{Arena, Id, Idx, idx_span::IdxSpanIter};
use append_only_vec::AppendOnlyVec;
use core::{
    marker::PhantomData,
    ops::{Index, IndexMut},
};

/// A index-based arena.
///
/// [`Arena`] provides a mechanism to allocate objects and refer to them by a
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
    /// assert!(arena.is_empty());
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
    /// If you hnadle this case, use [`Arena::try_alloc`] instead.
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

    /// Fallible version of [`Arena::alloc`].
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
