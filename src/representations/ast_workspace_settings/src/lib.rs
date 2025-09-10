use arena::{Idx, new_id};
use compiler_version::AdeptVersion;
use fs_tree::FsNodeId;
use primitives::CIntegerAssumptions;
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct Settings {
    pub adept_version: AdeptVersion,
    pub debug_skip_merging_helper_exprs: bool,
    pub imported_namespaces: Vec<Box<str>>,
    pub c_integer_assumptions: CIntegerAssumptions,
    pub namespace_to_dependency: HashMap<String, Vec<String>>,
    pub dependency_to_module: HashMap<String, FsNodeId>,
}

impl Default for Settings {
    fn default() -> Settings {
        Settings {
            adept_version: AdeptVersion::CURRENT,
            debug_skip_merging_helper_exprs: false,
            imported_namespaces: vec![],
            c_integer_assumptions: CIntegerAssumptions::default(),
            namespace_to_dependency: HashMap::new(),
            dependency_to_module: HashMap::new(),
        }
    }
}

impl Settings {
    pub fn c_integer_assumptions(&self) -> CIntegerAssumptions {
        self.c_integer_assumptions
    }
}

new_id!(SettingsId, u64);
pub type SettingsRef = Idx<SettingsId, Settings>;
