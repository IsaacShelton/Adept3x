use super::{AstFile, CIntegerAssumptions};
use crate::{
    iter_ext::MapSecond,
    source_files::SourceFiles,
    version::AdeptVersion,
    workspace::fs::{Fs, FsNodeId},
};
use append_only_vec::AppendOnlyVec;
use indexmap::IndexMap;
use std::collections::{HashMap, VecDeque};

#[derive(Debug)]
pub struct AstWorkspace<'a> {
    pub fs: Fs,
    pub files: IndexMap<FsNodeId, AstFile>,
    pub source_files: &'a SourceFiles,
    pub settings: Box<[Settings]>,
    pub module_folders: HashMap<FsNodeId, SettingsId>,
}

impl<'a> AstWorkspace<'a> {
    pub const DEFAULT_SETTINGS_ID: SettingsId = SettingsId(0);

    pub fn get_owning_module(&self, fs_node_id: FsNodeId) -> Option<FsNodeId> {
        let mut fs_node_id = fs_node_id;

        loop {
            if self.module_folders.contains_key(&fs_node_id) {
                return Some(fs_node_id);
            }

            if let Some(parent) = self.fs.get(fs_node_id).parent {
                fs_node_id = parent;
            } else {
                break;
            }
        }

        None
    }

    pub fn new(
        fs: Fs,
        files: IndexMap<FsNodeId, AstFile>,
        source_files: &'a SourceFiles,
        module_folders_settings: Option<HashMap<FsNodeId, Settings>>,
    ) -> Self {
        let mut override_settings = HashMap::new();

        // Construct settings mappings
        let settings = AppendOnlyVec::new();

        assert_eq!(
            settings.push(Settings::default()),
            Self::DEFAULT_SETTINGS_ID.0
        );

        for (fs_node_id, module) in module_folders_settings.into_iter().flatten() {
            override_settings.insert(fs_node_id, SettingsId(settings.push(module)));
        }

        let mut workspace = Self {
            fs,
            files,
            source_files,
            settings: settings.into_vec().into_boxed_slice(),
            module_folders: override_settings,
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
                .module_folders
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
    pub adept_version: AdeptVersion,
    pub debug_skip_merging_helper_exprs: bool,
    pub imported_namespaces: Vec<Box<str>>,
    pub assume_int_at_least_32_bits: bool,
}

impl Default for Settings {
    fn default() -> Settings {
        Settings {
            adept_version: AdeptVersion::CURRENT,
            debug_skip_merging_helper_exprs: false,
            imported_namespaces: vec![],
            assume_int_at_least_32_bits: true,
        }
    }
}

impl Settings {
    pub fn c_integer_assumptions(&self) -> CIntegerAssumptions {
        CIntegerAssumptions {
            int_at_least_32_bits: self.assume_int_at_least_32_bits,
        }
    }
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
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
