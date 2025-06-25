mod builder;
mod cursor;
mod flatten;
mod graphviz;
mod label;
mod node;
use arena::{Arena, Id, Idx, new_id_with_niche};
pub use flatten::*;
pub use node::*;
use source_files::Source;
use std::{collections::HashMap, fmt::Debug};

new_id_with_niche!(ConstEvalId, u64);

pub type ConstEvalRef = Idx<ConstEvalId, ConstEval>;

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct ConstEval {
    context: HashMap<String, Vec<SymbolBinding>>,
    cfg: UntypedCfg,
}

#[derive(Clone, Debug)]
pub struct SymbolBinding {
    pub symbol: SymbolRef,
    pub source: Source,
}

pub type SymbolRef = ConstEvalRef;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum IsValue {
    RequireValue,
    NeglectValue,
}

new_id_with_niche!(NodeId, u64);
pub type NodeRef = Idx<NodeId, Node>;

#[derive(Clone)]
pub struct UntypedCfg {
    pub ordered_nodes: Arena<NodeId, Node>,
}

impl Debug for UntypedCfg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_map().entries(self.ordered_nodes.iter()).finish()
    }
}

#[derive(Clone, Debug)]
pub enum CfgScopingError {
    InconsistentScoping(NodeRef),
    UnclosedScopes,
}

impl UntypedCfg {
    #[inline]
    pub fn start(&self) -> NodeRef {
        assert_ne!(self.len(), 0);
        unsafe { NodeRef::from_raw(NodeId::from_usize(0)) }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.ordered_nodes.len()
    }
}
