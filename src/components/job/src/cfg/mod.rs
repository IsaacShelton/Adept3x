mod builder;
mod const_eval;
mod cursor;
mod flatten;
mod graphviz;
mod human_label;
mod node;

use arena::{Arena, Id, Idx, new_id_with_niche};
pub use const_eval::*;
pub use cursor::*;
use diagnostics::ErrorDiagnostic;
pub use flatten::*;
pub use node::*;
use source_files::Source;
use std::{collections::HashMap, fmt::Debug};

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum IsValue {
    RequireValue,
    NeglectValue,
}

new_id_with_niche!(NodeId, u64);
pub type NodeRef = Idx<NodeId, Node>;

#[derive(Clone, Debug)]
pub struct Label {
    pub name: String,
    pub source: Source,
    pub node_ref: NodeRef,
}

#[derive(Clone)]
pub struct UntypedCfg {
    pub ordered_nodes: Arena<NodeId, Node>,
    pub labels: Vec<Label>,
}

impl Debug for UntypedCfg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_map().entries(self.ordered_nodes.iter()).finish()
    }
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

    pub fn finalize_gotos(mut self) -> Result<Self, ErrorDiagnostic> {
        let mut labels = HashMap::with_capacity(self.labels.len());

        for label in std::mem::take(&mut self.labels).into_iter() {
            if labels.contains_key(&label.name) {
                return Err(ErrorDiagnostic::new(
                    format!("Duplicate label '@{}@'", &label.name),
                    label.source,
                ));
            }

            assert_eq!(labels.insert(label.name, label.node_ref), None);
        }

        for node in self.ordered_nodes.values_mut() {
            match &mut node.kind {
                NodeKind::Sequential(SequentialNode {
                    kind: SequentialNodeKind::DirectGoto(label_name),
                    next,
                }) => {
                    let Some(destination) = labels.get(label_name) else {
                        return Err(ErrorDiagnostic::new(
                            format!("Undefined label '@{}@'", label_name),
                            node.source,
                        ));
                    };

                    *next = Some(*destination);
                }
                _ => (),
            }
        }

        Ok(self)
    }
}
