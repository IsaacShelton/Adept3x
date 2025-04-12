use arena::Idx;
use ast_workspace_settings::{Settings, SettingsId};
use fs_tree::FsNodeId;

pub struct ConfigureJob {
    pub fs_node_id: FsNodeId,
    pub settings: Idx<SettingsId, Settings>,
}

impl ConfigureJob {
    pub fn new(fs_node_id: FsNodeId, settings: Idx<SettingsId, Settings>) -> Self {
        Self {
            fs_node_id,
            settings,
        }
    }
}
