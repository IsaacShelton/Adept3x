use crate::path::normalized_path_segments;
use append_only_vec::AppendOnlyVec;
use once_map::OnceMap;
use std::{
    ffi::{OsStr, OsString},
    path::Path,
    sync::atomic::{AtomicU64, Ordering},
};

#[derive(Debug)]
pub struct Fs {
    pub arena: AppendOnlyVec<FsNode>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FsNodeId(usize);

impl FsNodeId {
    pub fn parent(self, fs: &Fs) -> Option<&FsNode> {
        fs.get(self).parent.map(|parent| fs.get(parent))
    }
}

impl Fs {
    pub const ROOT: FsNodeId = FsNodeId(0);

    pub fn new() -> Self {
        let arena = AppendOnlyVec::new();

        let root_node = FsNode {
            node_type: FsNodeType::Directory,
            children: OnceMap::new(),
            last_modified_ms: 0.into(),
            parent: None,
            segment: OsString::new().into_boxed_os_str(),
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

    pub fn insert(&self, path: &Path, last_modified_ms: Option<u64>) -> Option<FsNodeId> {
        self.root().deep_insert(
            &self,
            &normalized_path_segments(path),
            last_modified_ms.unwrap_or(0),
            Some(Self::ROOT),
        )
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
    pub segment: Box<OsStr>,
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
    ) -> Option<FsNodeId> {
        let Some((path_segment, rest)) = components.split_first() else {
            return None;
        };

        // Keep track of latest modified date per folder as well
        // SAFETY: This is okay, as we promise to never read from this
        // without synchronizing
        self.last_modified_ms
            .fetch_max(last_modified_ms, Ordering::Relaxed);

        // ===== Insert node into sub-tree =====

        let make_key = |path_segment: &OsStr| path_segment.to_os_string();

        let make_value = |path_segment: &OsString| {
            let segment = path_segment.clone().into_boxed_os_str();

            fs.new_node(FsNode {
                last_modified_ms: last_modified_ms.into(),
                node_type: rest
                    .is_empty()
                    .then_some(FsNodeType::File)
                    .unwrap_or(FsNodeType::Directory),
                children: OnceMap::new(),
                parent,
                segment,
            })
        };

        let and_then_do = |_path_segment: &OsString, id: &FsNodeId| {
            let node = fs.get(*id);

            // SAFETY: This is okay, as we promise to never read from this
            // without synchronizing
            node.last_modified_ms
                .fetch_max(last_modified_ms, Ordering::Relaxed);

            node.deep_insert(fs, rest, last_modified_ms, Some(*id))
                .unwrap_or(*id)
        };

        Some(
            self.children
                .map_insert_ref(*path_segment, make_key, make_value, and_then_do),
        )
    }
}
