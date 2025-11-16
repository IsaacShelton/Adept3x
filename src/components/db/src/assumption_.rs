use smallvec::{SmallVec, smallvec};
use std::{
    collections::{HashSet, VecDeque},
    fmt::Debug,
    sync::Arc,
};
use std_ext::SmallVec2;

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum Assumption {
    AlwaysViolated,
}

// Assumptions are a tree of assumptions, which may contain duplicates.
// This is okay, as we will deduplicate during checking.
// There will also never be any cycles, as this is an immutable data structure.
#[derive(Clone)]
pub struct Assumptions {
    inner: Option<Arc<AssumptionNode>>,
}

impl Assumptions {
    pub fn empty() -> Self {
        Self { inner: None }
    }

    pub fn new(data: Assumption) -> Self {
        Self {
            inner: Some(Arc::new(AssumptionNode::new(data, smallvec![]))),
        }
    }

    pub fn concat(&self, other: Self) -> Self {
        let Some(self_inner) = &self.inner else {
            return other;
        };

        let Some(other_inner) = other.inner else {
            return self.clone();
        };

        Self {
            inner: Some(Arc::new(AssumptionNode::join(smallvec![
                self_inner.clone(),
                other_inner
            ]))),
        }
    }

    pub fn traverse(&self, mut f: impl FnMut(&Assumption) -> Result<(), ()>) -> Result<(), ()> {
        self.inner
            .as_ref()
            .map(|start| start.traverse(&mut f))
            .unwrap_or(Ok(()))
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_none()
    }
}

impl Debug for Assumptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut stack = Vec::<(&AssumptionNode, usize)>::new();

        if let Some(root) = self.inner.as_ref() {
            stack.push((root, 0));
        }

        while let Some((node, indentation)) = stack.pop() {
            stack.extend(
                node.parents
                    .iter()
                    .rev()
                    .map(|parent| (parent.as_ref(), indentation + 2)),
            );

            for _ in 0..indentation {
                write!(f, " ")?;
            }

            if let Some(data) = &node.data {
                write!(f, "{:?}", data)?;
            } else {
                write!(f, "(parent)")?;
            }

            if stack.last().is_some() {
                writeln!(f, "")?;
            }
        }

        Ok(())
    }
}

pub struct AssumptionNode {
    parents: SmallVec2<Arc<AssumptionNode>>,
    data: Option<Assumption>,
}

impl AssumptionNode {
    pub fn new(data: Assumption, parents: SmallVec2<Arc<AssumptionNode>>) -> Self {
        Self {
            data: Some(data),
            parents,
        }
    }

    pub fn join(parents: SmallVec2<Arc<AssumptionNode>>) -> Self {
        Self {
            parents,
            data: None,
        }
    }

    pub fn traverse(&self, f: &mut impl FnMut(&Assumption) -> Result<(), ()>) -> Result<(), ()> {
        if let Some(assumption) = &self.data {
            f(assumption)?;
        }

        for parent in self.parents.iter() {
            parent.traverse(f)?;
        }

        Ok(())
    }
}
