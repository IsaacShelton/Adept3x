use itertools::Itertools;
use std::{cmp::Ordering, collections::VecDeque, fmt::Debug, hash::Hash};

#[derive(Clone, Debug)]
pub struct TopN<T> {
    items: VecDeque<T>,
    capacity: usize,
}

impl<T> TopN<T> {
    pub fn new(capacity: usize) -> Self {
        Self {
            items: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn cap(&self) -> usize {
        self.capacity
    }

    pub fn from_iter(
        capacity: usize,
        iter: impl IntoIterator<Item = T>,
        comparator: impl Fn(&T, &T) -> Ordering,
    ) -> Self {
        Self {
            items: iter
                .into_iter()
                .k_smallest_by(capacity, &comparator)
                .sorted_by(&comparator)
                .collect(),
            capacity,
        }
    }

    // NOTE: We intentionally avoid using `Ord` here so that each `T` doesn't have to maintain a
    // reference to the context necessary to perform the comparison
    pub fn push(
        &mut self,
        new_item: T,
        mut comparator: impl FnMut(&T, &T) -> Ordering,
    ) -> &mut Self {
        // Normally `n` should be small, so we will do naive way for now

        // Find existing item that new item should come after
        if let Some(i) = self
            .items
            .iter()
            .enumerate()
            .rev()
            .find_map(|(i, item)| comparator(&new_item, item).is_ge().then_some(i))
        {
            // Don't do anything if existing items are more important than new item
            if i + 1 >= self.capacity {
                return self;
            }

            // Shave off back if too many items
            if self.items.len() >= self.capacity {
                self.items.pop_back();
            }

            // Insert new item into place
            self.items.insert(i + 1, new_item);
            return self;
        }

        // Otherwise, nothing is more important than the new item,
        // so put it at the front.

        // Shave off back if too many items
        if self.items.len() >= self.capacity {
            self.items.pop_back();
        }

        // Insert new item as new most important item
        self.items.push_front(new_item);
        self
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.items.iter()
    }
}

impl<T> IntoIterator for TopN<T> {
    type Item = T;
    type IntoIter = <VecDeque<T> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.items.into_iter()
    }
}

impl<T: Hash> Hash for TopN<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.items.hash(state);
    }
}

impl<T: PartialEq> PartialEq for TopN<T> {
    fn eq(&self, other: &Self) -> bool {
        self.items == other.items
    }
}

impl<T: Eq> Eq for TopN<T> {}
