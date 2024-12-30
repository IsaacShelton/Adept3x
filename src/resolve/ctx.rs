use super::{func_haystack::FuncHaystack, job::FuncJob};
use crate::{asg, workspace::fs::FsNodeId};
use indexmap::IndexMap;
use std::collections::{HashMap, VecDeque};

pub struct ResolveCtx {
    pub jobs: VecDeque<FuncJob>,
    pub func_haystacks: IndexMap<FsNodeId, FuncHaystack>,
    pub public_funcs: HashMap<FsNodeId, HashMap<String, Vec<asg::FuncRef>>>,
    pub types_in_modules: HashMap<FsNodeId, HashMap<String, asg::TypeDecl>>,
    pub globals_in_modules: HashMap<FsNodeId, HashMap<String, asg::GlobalVarDecl>>,
    pub helper_exprs_in_modules: HashMap<FsNodeId, HashMap<String, asg::HelperExprDecl>>,
    pub trait_haystacks: HashMap<FsNodeId, HashMap<String, asg::TraitRef>>,
    pub impls_in_modules: HashMap<FsNodeId, HashMap<String, asg::ImplRef>>,
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
            trait_haystacks: HashMap::new(),
            impls_in_modules: HashMap::new(),
        }
    }
}
