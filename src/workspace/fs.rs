use once_map::OnceMap;
use std::{
    ffi::{OsStr, OsString},
    sync::atomic::{AtomicU64, Ordering},
};

#[derive(Copy, Clone, Debug)]
pub enum FsNodeType {
    Directory,
    File,
}

#[derive(Debug)]
pub struct FsNode {
    pub node_type: FsNodeType,
    pub children: OnceMap<OsString, FsNode>,
    pub last_modified_ms: AtomicU64,
}

impl FsNode {
    pub fn deep_insert(&self, components: &[&OsStr], last_modified_ms: u64) {
        let Some((path_segment, rest)) = components.split_first() else {
            return;
        };

        // Keep track of latest modified date per folder as well
        self.last_modified_ms
            .fetch_max(last_modified_ms, Ordering::SeqCst);

        // ===== Insert node into sub-tree =====

        let make_key = |path_segment: &OsStr| path_segment.to_os_string();

        let make_value = |_path_segment: &OsString| FsNode {
            last_modified_ms: last_modified_ms.into(),
            node_type: rest
                .is_empty()
                .then_some(FsNodeType::File)
                .unwrap_or(FsNodeType::Directory),
            children: OnceMap::new(),
        };

        let and_then_do = |_path_segment: &OsString, fs_node: &FsNode| {
            fs_node.deep_insert(
                rest,
                fs_node
                    .last_modified_ms
                    .fetch_max(last_modified_ms, Ordering::Relaxed),
            )
        };

        self.children
            .map_insert_ref(*path_segment, make_key, make_value, and_then_do);
    }
}
