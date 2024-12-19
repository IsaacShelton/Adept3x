use super::{function_haystack::FunctionHaystack, job::FuncJob};
use crate::{
    resolved::{self, Implementations},
    workspace::fs::FsNodeId,
};
use indexmap::IndexMap;
use std::collections::{HashMap, VecDeque};

pub struct ResolveCtx<'a> {
    pub jobs: VecDeque<FuncJob>,
    pub function_haystacks: IndexMap<FsNodeId, FunctionHaystack>,
    pub public_functions: HashMap<FsNodeId, HashMap<String, Vec<resolved::FunctionRef>>>,
    pub types_in_modules: HashMap<FsNodeId, HashMap<String, resolved::TypeDecl>>,
    pub globals_in_modules: HashMap<FsNodeId, HashMap<String, resolved::GlobalVarDecl>>,
    pub helper_exprs_in_modules: HashMap<FsNodeId, HashMap<String, resolved::HelperExprDecl>>,
    pub implementations: &'a Implementations,
}

impl<'a> ResolveCtx<'a> {
    pub fn new(implementations: &'a Implementations) -> Self {
        Self {
            jobs: Default::default(),
            function_haystacks: Default::default(),
            public_functions: HashMap::new(),
            types_in_modules: HashMap::new(),
            globals_in_modules: HashMap::new(),
            helper_exprs_in_modules: HashMap::new(),
            implementations,
        }
    }
}
