use crate::Error;
use serde::{Deserialize, Serialize};
use smallvec::{SmallVec, smallvec};
use std::sync::Arc;
use top_n::TopN;

#[derive(Clone, Debug, Default, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopErrors {
    inner: Option<Arc<TopErrorsNode>>,
}

impl TopErrors {
    pub fn new(errors: impl Iterator<Item = Error>) -> Self {
        Self {
            inner: TopErrorsNode::new(errors).map(Arc::new),
        }
    }

    pub fn new_one(error: Error) -> Self {
        Self {
            inner: TopErrorsNode::new(std::iter::once(error)).map(Arc::new),
        }
    }

    pub fn iter_unordered<'a>(&'a self) -> TopErrorsUnorderedIter<'a> {
        self.inner
            .as_ref()
            .map(|inner| inner.iter_unordered())
            .unwrap_or_default()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_none()
    }
}

impl From<TopErrorsNode> for TopErrors {
    fn from(value: TopErrorsNode) -> Self {
        Self {
            inner: Some(Arc::new(value)),
        }
    }
}

impl From<Arc<TopErrorsNode>> for TopErrors {
    fn from(value: Arc<TopErrorsNode>) -> Self {
        Self { inner: Some(value) }
    }
}

#[derive(Clone, Debug, Default, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopErrorsNode {
    // Invariant: there must always be at least one error
    // in the errors node.
    errors: SmallVec<[Error; 1]>,

    // We guarantee that this will always be acyclic by construction
    parent: Option<Arc<TopErrorsNode>>,
}

impl TopErrorsNode {
    pub fn new(errors: impl IntoIterator<Item = Error>) -> Option<Self> {
        let list: SmallVec<[Error; 1]> = errors.into_iter().collect();

        if list.is_empty() {
            None
        } else {
            Some(Self {
                errors: list,
                parent: None,
            })
        }
    }

    pub fn new_with_parent(
        errors: impl IntoIterator<Item = Error>,
        parent: Arc<TopErrorsNode>,
    ) -> Option<Self> {
        let list: SmallVec<[Error; 1]> = errors.into_iter().collect();

        if list.is_empty() {
            None
        } else {
            Some(Self {
                errors: list,
                parent: Some(parent),
            })
        }
    }

    pub fn with(mut self: Arc<Self>, new_errors: impl Iterator<Item = Error>) -> Arc<Self> {
        Arc::make_mut(&mut self).errors.extend(new_errors);
        self
    }

    pub fn top(&self, n: usize) -> TopN<&Error> {
        TopN::from_iter(n, self.iter_unordered(), |a, b| a.cmp(b))
    }

    pub fn iter_unordered<'a>(&'a self) -> TopErrorsUnorderedIter<'a> {
        TopErrorsUnorderedIter::new(self)
    }

    pub fn is_empty(&self) -> bool {
        self.iter_unordered().next().is_none()
    }
}

#[derive(Default)]
pub struct TopErrorsUnorderedIter<'a> {
    stack: SmallVec<[&'a TopErrorsNode; 10]>,
    slice: &'a [Error],
}

impl<'a> TopErrorsUnorderedIter<'a> {
    pub fn new(top_errors: &'a TopErrorsNode) -> Self {
        Self {
            stack: smallvec![top_errors],
            slice: &[],
        }
    }
}

impl<'a> Iterator for TopErrorsUnorderedIter<'a> {
    type Item = &'a Error;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some((first, rest)) = self.slice.split_first() {
                self.slice = rest;
                return Some(first);
            }

            let node = self.stack.pop()?;
            self.slice = &node.errors[..];

            if let Some(node) = &node.parent {
                self.stack.push(node.as_ref());
            }
        }
    }
}
