use itertools::Itertools;
use std::cmp::Ordering;

#[derive(Clone, Debug)]
pub struct TopN<T> {
    items: Vec<T>,
    capacity: usize,
}

impl<T> TopN<T> {
    pub fn new(capacity: usize) -> Self {
        Self {
            items: Vec::with_capacity(capacity),
            capacity,
        }
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn from_iter<CTX>(
        capacity: usize,
        iter: impl IntoIterator<Item = T>,
        ctx: &CTX,
        comparator: impl Fn(&T, &T, &CTX) -> Ordering,
    ) -> Self {
        Self {
            items: iter
                .into_iter()
                .k_smallest_by(capacity, |a, b| comparator(a, b, ctx))
                .collect(),
            capacity,
        }
    }

    // NOTE: We intentionally avoid using `Ord` here so that each `T` doesn't have to maintain a
    // reference to the context necessary to perform the comparison
    pub fn push(&mut self, new_item: T, mut comparator: impl FnMut(&T, &T) -> Ordering) {
        // Normally `n` should be small, so we will do naive way for now

        // Find insert position within array
        if let Some(i) =
            self.items.iter().enumerate().rev().find_map(|(i, item)| {
                (comparator(&new_item, item) != Ordering::Greater).then_some(i)
            })
        {
            self.items.insert(i + 1, new_item);
            if self.items.len() > self.capacity {
                self.items.pop();
            }
            return;
        }

        // Otherwise try to add to end
        if self.items.len() < self.capacity {
            self.items.push(new_item);
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.items.iter()
    }
}

impl<T> IntoIterator for TopN<T> {
    type Item = T;
    type IntoIter = <Vec<T> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.items.into_iter()
    }
}
