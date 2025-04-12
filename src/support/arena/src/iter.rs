use super::{Id, Idx};
use core::{
    iter::{Enumerate, FusedIterator},
    marker::PhantomData,
    slice,
};

macro_rules! iter_iterator_impls {
    ($ty:ty, type Item = $item_ty:ty;) => {
        impl<'a, K: Id, V> Iterator for $ty {
            type Item = $item_ty;

            #[inline]
            fn next(&mut self) -> Option<Self::Item> {
                let (id, value) = self.iter.next().map(|(k, v)| (K::from_usize(k), v))?;
                Some((
                    Idx {
                        raw: id,
                        phantom: PhantomData,
                    },
                    value,
                ))
            }

            #[inline]
            fn size_hint(&self) -> (usize, Option<usize>) {
                self.iter.size_hint()
            }

            #[inline]
            fn count(self) -> usize {
                self.iter.count()
            }

            #[inline]
            fn last(self) -> Option<Self::Item> {
                self.iter.last().map(|(k, v)| {
                    (
                        Idx {
                            raw: K::from_usize(k),
                            phantom: PhantomData,
                        },
                        v,
                    )
                })
            }

            #[inline]
            fn nth(&mut self, n: usize) -> Option<Self::Item> {
                let (id, value) = self.iter.nth(n).map(|(k, v)| (K::from_usize(k), v))?;
                Some((
                    Idx {
                        raw: id,
                        phantom: PhantomData,
                    },
                    value,
                ))
            }
        }

        impl<'a, K: Id, V> DoubleEndedIterator for $ty {
            #[inline]
            fn next_back(&mut self) -> Option<Self::Item> {
                let (id, value) = self.iter.next_back().map(|(k, v)| (K::from_usize(k), v))?;
                Some((
                    Idx {
                        raw: id,
                        phantom: PhantomData,
                    },
                    value,
                ))
            }
        }

        impl<'a, K: Id, V> ExactSizeIterator for $ty {
            #[inline]
            fn len(&self) -> usize {
                self.iter.len()
            }
        }

        impl<'a, K: Id, V> FusedIterator for $ty {}
    };
}

pub struct Iter<'a, K: Id, V> {
    pub(crate) iter: Enumerate<slice::Iter<'a, V>>,
    pub(crate) phantom: PhantomData<K>,
}

iter_iterator_impls! {
    Iter<'a, K, V>,
    type Item = (Idx<K, V>, &'a V);
}

impl<K: Id, V> Clone for Iter<'_, K, V> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            iter: self.iter.clone(),
            phantom: PhantomData,
        }
    }
}

pub struct IterMut<'a, K: Id, V> {
    pub(crate) iter: Enumerate<slice::IterMut<'a, V>>,
    pub(crate) phantom: PhantomData<K>,
}

iter_iterator_impls! {
    IterMut<'a, K, V>,
    type Item = (Idx<K, V>, &'a mut V);
}

pub struct IntoIter<K: Id, V> {
    pub(crate) iter: Enumerate<alloc::vec::IntoIter<V>>,
    pub(crate) phantom: PhantomData<K>,
}

iter_iterator_impls! {
    IntoIter<K, V>,
    type Item = (Idx<K, V>, V);
}

impl<K: Id, V: Clone> Clone for IntoIter<K, V> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            iter: self.iter.clone(),
            phantom: PhantomData,
        }
    }
}
