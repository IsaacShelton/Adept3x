use super::Id;
use core::{iter::FusedIterator, marker::PhantomData, slice};

macro_rules! values_iterator_impls {
    ($ty:ty, type Item = $item_ty:ty;) => {
        impl<'a, K: Id, V> Iterator for $ty {
            type Item = $item_ty;

            #[inline]
            fn next(&mut self) -> Option<Self::Item> {
                self.iter.next()
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
                self.iter.last()
            }

            #[inline]
            fn nth(&mut self, n: usize) -> Option<Self::Item> {
                self.iter.nth(n)
            }
        }

        impl<'a, K: Id, V> DoubleEndedIterator for $ty {
            #[inline]
            fn next_back(&mut self) -> Option<Self::Item> {
                self.iter.next_back()
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

pub struct Values<'a, K: Id, V> {
    pub(crate) iter: slice::Iter<'a, V>,
    pub(crate) phantom: PhantomData<K>,
}

values_iterator_impls! {
    Values<'a, K, V>,
    type Item = &'a V;
}

impl<K: Id, V> Clone for Values<'_, K, V> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            iter: self.iter.clone(),
            phantom: PhantomData,
        }
    }
}

pub struct ValuesMut<'a, K: Id, V> {
    pub(crate) iter: slice::IterMut<'a, V>,
    pub(crate) phantom: PhantomData<K>,
}

values_iterator_impls! {
    ValuesMut<'a, K, V>,
    type Item = &'a mut V;
}
