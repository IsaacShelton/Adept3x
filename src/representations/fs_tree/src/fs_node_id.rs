use crate::{Fs, FsNode};
use arena::{Id, NewId};

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FsNodeId(pub(crate) usize);

impl FsNodeId {
    pub fn parent(self, fs: &Fs) -> Option<&FsNode> {
        fs.get(self).parent.map(|parent| fs.get(parent))
    }
}

impl Id for FsNodeId {
    const MAX: usize = usize::MAX;

    fn from_usize(idx: usize) -> Self {
        FsNodeId(idx)
    }

    fn into_usize(self) -> usize {
        self.0
    }

    fn successor(self) -> Self {
        Self(self.0 + 1)
    }
}

impl NewId for FsNodeId {}
