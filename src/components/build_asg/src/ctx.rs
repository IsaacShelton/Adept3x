use super::{func_haystack::FuncHaystack, job::FuncJob};
use fs_tree::FsNodeId;
use indexmap::IndexMap;
use std::collections::{HashMap, VecDeque};

pub struct ResolveCtx {
    pub jobs: VecDeque<FuncJob>,
    pub func_haystacks: IndexMap<FsNodeId, FuncHaystack>,
    pub public_funcs: HashMap<FsNodeId, HashMap<String, Vec<asg::FuncRef>>>,
    pub types_in_modules: HashMap<FsNodeId, HashMap<String, asg::TypeDecl>>,
    pub globals_in_modules: HashMap<FsNodeId, HashMap<String, asg::GlobalDecl>>,
    pub helper_exprs_in_modules: HashMap<FsNodeId, HashMap<String, asg::HelperExprDecl>>,
    pub impls_in_modules: HashMap<FsNodeId, HashMap<String, asg::ImplDecl>>,
}

impl ResolveCtx {
    pub fn new() -> Self {
        Self {
            jobs: Default::default(),
            func_haystacks: Default::default(),
            public_funcs: HashMap::new(),
            types_in_modules: HashMap::new(),
            globals_in_modules: HashMap::new(),
            helper_exprs_in_modules: HashMap::new(),
            impls_in_modules: HashMap::new(),
        }
    }
}
