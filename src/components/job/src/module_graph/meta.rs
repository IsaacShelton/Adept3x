use crate::module_graph::ModuleGraphRef;
use target::Target;

#[derive(Clone, Debug)]
pub struct ModuleGraphMeta {
    // Human-readable title for this module graph.
    pub title: &'static str,

    // Whether this module graph is meant for compile-time code evaluation.
    pub self_ref: ModuleGraphRef,

    // The target for this module graph
    pub target: Target,
}
