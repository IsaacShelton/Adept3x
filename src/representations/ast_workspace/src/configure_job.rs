use ast_workspace_settings::SettingsId;
use fs_tree::FsNodeId;

pub struct ConfigureJob {
    pub fs_node_id: FsNodeId,
    pub settings: SettingsId,
}

impl ConfigureJob {
    pub fn new(fs_node_id: FsNodeId, settings: SettingsId) -> Self {
        Self {
            fs_node_id,
            settings,
        }
    }
}
