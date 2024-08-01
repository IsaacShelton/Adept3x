use append_only_vec::AppendOnlyVec;
use once_map::OnceMap;
use std::{
    ffi::{OsStr, OsString},
    sync::atomic::{AtomicU64, Ordering},
};

#[derive(Debug)]
pub struct Fs {
    pub arena: AppendOnlyVec<FsNode>,
}

#[derive(Copy, Clone, Debug)]
pub struct FsNodeId(usize);

impl FsNodeId {
    pub fn parent(self, fs: &Fs) -> Option<&FsNode> {
        fs.get(self).parent.map(|parent| fs.get(parent))
    }
}

impl Fs {
    const ROOT: FsNodeId = FsNodeId(0);

    pub fn new() -> Self {
        let arena = AppendOnlyVec::new();

        let root_node = FsNode {
            node_type: FsNodeType::Directory,
            children: OnceMap::new(),
            last_modified_ms: 0.into(),
            parent: None,
        };

        // We assume that the root is at index 0
        assert_eq!(arena.push(root_node), Self::ROOT.0);

        Self { arena }
    }

    pub fn root(&self) -> &FsNode {
        &self.arena[Self::ROOT.0]
    }

    pub fn new_node(&self, node: FsNode) -> FsNodeId {
        FsNodeId(self.arena.push(node))
    }

    pub fn insert(&self, components: &[&OsStr], last_modified_ms: u64) {
        self.root()
            .deep_insert(&self, components, last_modified_ms, Some(Self::ROOT));
    }

    pub fn get(&self, id: FsNodeId) -> &FsNode {
        &self.arena[id.0]
    }
}

#[derive(Copy, Clone, Debug)]
pub enum FsNodeType {
    Directory,
    File,
}

#[derive(Debug)]
pub struct FsNode {
    pub node_type: FsNodeType,
    pub children: OnceMap<OsString, FsNodeId>,
    pub last_modified_ms: AtomicU64,
    pub parent: Option<FsNodeId>,
}

impl FsNode {
    pub fn parent<'a>(&self, fs: &'a Fs) -> Option<&'a FsNode> {
        self.parent.map(|parent| fs.get(parent))
    }

    pub fn deep_insert(
        &self,
        fs: &Fs,
        components: &[&OsStr],
        last_modified_ms: u64,
        parent: Option<FsNodeId>,
    ) {
        let Some((path_segment, rest)) = components.split_first() else {
            return;
        };

        // Keep track of latest modified date per folder as well
        self.last_modified_ms
            .fetch_max(last_modified_ms, Ordering::SeqCst);

        // ===== Insert node into sub-tree =====

        let make_key = |path_segment: &OsStr| path_segment.to_os_string();

        let make_value = |_path_segment: &OsString| {
            fs.new_node(FsNode {
                last_modified_ms: last_modified_ms.into(),
                node_type: rest
                    .is_empty()
                    .then_some(FsNodeType::File)
                    .unwrap_or(FsNodeType::Directory),
                children: OnceMap::new(),
                parent,
            })
        };

        let and_then_do = |_path_segment: &OsString, id: &FsNodeId| {
            let node = fs.get(*id);

            node.deep_insert(
                fs,
                rest,
                node.last_modified_ms
                    .fetch_max(last_modified_ms, Ordering::Relaxed),
                Some(*id),
            )
        };

        self.children
            .map_insert_ref(*path_segment, make_key, make_value, and_then_do);
    }
}
