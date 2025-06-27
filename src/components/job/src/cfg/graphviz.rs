use super::{Node, NodeId, NodeKind, NodeRef, UntypedCfg, human_label::HumanLabel};
use arena::{Id, Idx};
use indexmap::IndexMap;
use std::collections::VecDeque;

impl UntypedCfg {
    pub fn write_to_graphviz_file(&self, filename: &str) {
        let mut content = String::new();
        content.push_str("digraph G {\n");

        let mut edges = IndexMap::<(NodeRef, NodeRef), &'static str>::new();
        let mut queue = VecDeque::new();
        queue.push_back(unsafe { NodeRef::from_raw(NodeId::from_usize(0)) });

        let mut explore = |queue: &mut VecDeque<Idx<NodeId, Node>>,
                           from: NodeRef,
                           to: Option<NodeRef>,
                           label: &'static str| {
            let Some(to) = to else {
                return;
            };

            if edges.contains_key(&(from, to)) {
                return;
            }

            edges.insert((from, to), label);
            queue.push_back(to);
        };

        while let Some(node_ref) = queue.pop_front() {
            let node = &self.nodes[node_ref];

            match &node.kind {
                NodeKind::Start(next) => {
                    assert!(node_ref.into_raw().into_usize() == 0);

                    if let Some(next) = next {
                        queue.push_back(*next);
                    }
                }
                NodeKind::Sequential(sequential_node) => {
                    explore(&mut queue, node_ref, sequential_node.next, "".into());
                }
                NodeKind::Branching(branch_node) => {
                    explore(&mut queue, node_ref, branch_node.when_true, "true".into());
                    explore(&mut queue, node_ref, branch_node.when_false, "false".into());
                }
                NodeKind::Scope(scope_node) => {
                    explore(&mut queue, node_ref, scope_node.inner, "always".into());
                    explore(
                        &mut queue,
                        node_ref,
                        scope_node.closed_at,
                        "closed_at".into(),
                    );
                }
                NodeKind::Terminating(_) => (),
            }
        }

        for (node_ref, node) in self.nodes.iter().skip(1) {
            content.push_str(&format!(
                "a{} [label=\"{:?}: {}\"];\n",
                node_ref.into_raw().into_usize(),
                node_ref.into_raw(),
                node.label().replace("\"", "<quote>")
            ));
        }

        content.push_str("start [shape=Mdiamond];\n");
        content.push_str("start -> a1;\n");

        for (edge, label) in edges.iter() {
            let mut attribs = String::new();

            if !label.is_empty() {
                attribs.push_str(&format!(" [label=\"{}\"]", label));
            }

            content.push_str(&format!(
                "a{} -> a{}{}\n",
                edge.0.into_raw().into_usize(),
                edge.1.into_raw().into_usize(),
                attribs,
            ));
        }

        for (node_ref, node) in &self.nodes {
            if let NodeKind::Terminating(_) = &node.kind {
                content.push_str(&format!(
                    "a{} [shape=Msquare];\n",
                    node_ref.into_raw().into_usize()
                ));
            }
        }

        content.push_str("}\n");

        use std::io::Write;
        let mut file = std::fs::File::create(filename).unwrap();
        file.write_all(content.as_bytes()).unwrap();
    }
}
