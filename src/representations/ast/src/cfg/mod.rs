mod builder;
mod cursor;
mod flatten;
mod graphviz;
mod label;
mod node;
use arena::{Arena, ArenaMap, Id, Idx, new_id_with_niche};
pub use flatten::*;
pub use node::*;
use source_files::Source;
use std::{
    collections::{HashMap, VecDeque},
    fmt::Debug,
};

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

    pub fn assert_valid_scoping(&self) {
        if let Err(err) = self.validate_scoping() {
            panic!("assert_validate_scoping failed! {:?}", err);
        }
    }

    pub fn validate_scoping(&self) -> Result<HashMap<(NodeRef, NodeRef), i32>, CfgScopingError> {
        assert!(self.ordered_nodes.len() > 0);

        let mut num_open_at = ArenaMap::<NodeId, i32>::new();
        let mut covered_edges = HashMap::<(NodeRef, NodeRef), i32>::new();
        let mut queue = VecDeque::<(NodeRef, i32)>::new();

        num_open_at.insert(NodeId::from_usize(0), 0);
        queue.push_back((unsafe { NodeRef::from_raw(NodeId::from_usize(0)) }, 0));

        let mut explore = |queue: &mut VecDeque<(NodeRef, i32)>,
                           from: NodeRef,
                           to: &Option<NodeRef>,
                           num_open: i32| {
            if let Some(to) = to {
                match covered_edges.insert((from, *to), num_open) {
                    Some(existing) => assert_eq!(num_open, existing),
                    None => queue.push_back((*to, num_open)),
                }
            }
        };

        while let Some((node_ref, num_open)) = queue.pop_front() {
            let node = &self.ordered_nodes[node_ref];

            match &node.kind {
                NodeKind::Start(next) => {
                    assert!(node_ref.into_raw().into_usize() == 0);

                    if let Some(next) = next {
                        queue.push_back((*next, num_open));
                    }
                }
                NodeKind::Sequential(sequential_node) => {
                    let new_value = match &sequential_node.kind {
                        SequentialNodeKind::OpenScope => num_open + 1,
                        SequentialNodeKind::CloseScope => num_open - 1,
                        _ => num_open,
                    };

                    if let Some(existing) = num_open_at.insert(node_ref.into_raw(), new_value) {
                        if existing != new_value {
                            return Err(CfgScopingError::InconsistentScoping(node_ref));
                        }
                    }

                    explore(&mut queue, node_ref, &sequential_node.next, new_value);
                }
                NodeKind::Branching(branch_node) => {
                    explore(&mut queue, node_ref, &branch_node.when_true, num_open);
                    explore(&mut queue, node_ref, &branch_node.when_false, num_open);
                }
                NodeKind::Terminating(_) => {
                    if num_open != 0 {
                        return Err(CfgScopingError::UnclosedScopes);
                    }
                }
            }
        }

        Ok(covered_edges)
    }
}
