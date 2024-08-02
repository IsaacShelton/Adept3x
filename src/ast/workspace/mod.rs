use super::AstFile;
use crate::{
    iter_ext::MapSecond,
    source_files::SourceFiles,
    version::AdeptVersion,
    workspace::fs::{Fs, FsNodeId},
};
use append_only_vec::AppendOnlyVec;
use indexmap::IndexMap;
use std::collections::VecDeque;

#[derive(Debug)]
pub struct AstWorkspace<'a> {
    pub fs: Fs,
    pub files: IndexMap<FsNodeId, AstFile>,
    pub source_files: &'a SourceFiles,
    pub settings: Box<[Settings]>,
    pub override_settings: IndexMap<FsNodeId, SettingsId>,
}

impl<'a> AstWorkspace<'a> {
    pub const DEFAULT_SETTINGS_ID: SettingsId = SettingsId(0);

    pub fn new(
        fs: Fs,
        files: IndexMap<FsNodeId, AstFile>,
        source_files: &'a SourceFiles,
        modules_settings: Option<IndexMap<FsNodeId, Settings>>,
    ) -> Self {
        let mut override_settings = IndexMap::new();

        // Construct settings mappings
        let settings = AppendOnlyVec::new();

        assert_eq!(
            settings.push(Settings {
                adept_version: AdeptVersion::CURRENT,
            }),
            Self::DEFAULT_SETTINGS_ID.0
        );

        for (fs_node_id, module) in modules_settings.into_iter().flatten() {
            override_settings.insert(fs_node_id, SettingsId(settings.push(module)));
        }

        let mut workspace = Self {
            fs,
            files,
            source_files,
            settings: settings.into_vec().into_boxed_slice(),
            override_settings,
        };
        workspace.configure();
        workspace
    }

    pub fn get_mut(&mut self, id: FsNodeId) -> Option<&mut AstFile> {
        self.files.get_mut(&id)
    }

    fn configure(&mut self) {
        let mut jobs = VecDeque::new();
        jobs.push_back(ConfigureJob::new(Fs::ROOT, Self::DEFAULT_SETTINGS_ID));

        while let Some(job) = jobs.pop_front() {
            let fs_node_id = job.fs_node_id;

            let settings = self
                .override_settings
                .get(&fs_node_id)
                .copied()
                .unwrap_or(job.settings);

            if let Some(ast_file) = self.files.get_mut(&fs_node_id) {
                ast_file.settings = Some(settings);
            }

            // SAFETY: `read_only_view` will never deadlock here because we promise
            // to not insert any children while viewing it on this same thread
            jobs.extend(
                self.fs
                    .get(fs_node_id)
                    .children
                    .read_only_view()
                    .iter()
                    .map_second()
                    .copied()
                    .map(|child_fs_node_id| ConfigureJob::new(child_fs_node_id, settings)),
            );
        }
    }
}

#[derive(Clone, Debug)]
pub struct Settings {
    adept_version: AdeptVersion,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SettingsId(pub usize);

struct ConfigureJob {
    fs_node_id: FsNodeId,
    settings: SettingsId,
}

impl ConfigureJob {
    pub fn new(fs_node_id: FsNodeId, settings: SettingsId) -> Self {
        Self {
            fs_node_id,
            settings,
        }
    }
}
