use super::{function_haystack::FunctionHaystack, job::FuncJob};
use crate::{resolved, workspace::fs::FsNodeId};
use indexmap::IndexMap;
use std::collections::{HashMap, VecDeque};

pub struct ResolveCtx {
    pub jobs: VecDeque<FuncJob>,
    pub function_haystacks: IndexMap<FsNodeId, FunctionHaystack>,
    pub public_functions: HashMap<FsNodeId, HashMap<String, Vec<resolved::FunctionRef>>>,
    pub types_in_modules: HashMap<FsNodeId, HashMap<String, resolved::TypeDecl>>,
    pub globals_in_modules: HashMap<FsNodeId, HashMap<String, resolved::GlobalVarDecl>>,
    pub helper_exprs_in_modules: HashMap<FsNodeId, HashMap<String, resolved::HelperExprDecl>>,
    pub trait_haystacks: HashMap<FsNodeId, HashMap<String, resolved::TraitRef>>,
    pub impl_haystacks: HashMap<FsNodeId, HashMap<String, resolved::TraitRef>>,
}

impl ResolveCtx {
    pub fn new() -> Self {
        Self {
            jobs: Default::default(),
            function_haystacks: Default::default(),
            public_functions: HashMap::new(),
            types_in_modules: HashMap::new(),
            globals_in_modules: HashMap::new(),
            helper_exprs_in_modules: HashMap::new(),
            trait_haystacks: HashMap::new(),
            impl_haystacks: HashMap::new(),
        }
    }
}
