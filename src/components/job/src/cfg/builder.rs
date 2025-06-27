use super::{
    BranchNode, ConstEval, ConstEvalId, ConstEvalRef, IsValue, Label, Node, NodeId, NodeKind,
    NodeRef, ScopeNode, SequentialNode, SequentialNodeKind, TerminatingNode, UntypedCfg, connect,
    cursor::{Cursor, CursorPosition},
    flatten_expr,
};
use arena::Arena;
use ast::{ConformBehavior, Expr};
use smallvec::smallvec;
use source_files::Source;
use std::collections::HashMap;
use std_ext::SmallVec2;

#[derive(Debug)]
pub struct Builder<'const_evals> {
    pub ordered_nodes: Arena<NodeId, Node>,
    pub labels: Vec<Label>,
    const_evals: &'const_evals mut Arena<ConstEvalId, ConstEval>,
}

#[allow(dead_code)]
impl<'const_evals> Builder<'const_evals> {
    #[must_use]
    pub fn new(
        const_evals: &'const_evals mut Arena<ConstEvalId, ConstEval>,
        source: Source,
    ) -> (Self, Cursor) {
        let mut ordered_nodes = Arena::new();

        let start_ref = ordered_nodes.alloc(Node {
            kind: NodeKind::Start(None),
            source,
        });

        (
            Self {
                ordered_nodes,
                const_evals,
                labels: Vec::new(),
            },
            CursorPosition {
                from: start_ref,
                edge_index: 0,
            }
            .into(),
        )
    }

    #[must_use]
    pub fn get(&self, idx: NodeRef) -> &Node {
        &self.ordered_nodes[idx]
    }

    #[must_use]
    pub fn get_mut(&mut self, idx: NodeRef) -> &mut Node {
        &mut self.ordered_nodes[idx]
    }

    #[must_use]
    pub fn push_sequential(
        &mut self,
        cursor: Cursor,
        node: SequentialNodeKind,
        source: Source,
    ) -> Cursor {
        let Some(position) = cursor.position else {
            return cursor;
        };

        let new_node_ref = self.ordered_nodes.alloc(Node {
            kind: NodeKind::Sequential(SequentialNode {
                kind: node,
                next: None,
            }),
            source,
        });

        connect(&mut self.ordered_nodes, position, new_node_ref);
        CursorPosition::new(new_node_ref, 0).into()
    }

    #[must_use]
    pub fn push_branch(
        &mut self,
        cursor: Cursor,
        condition: NodeRef,
        source: Source,
    ) -> (Cursor, Cursor) {
        let Some(position) = cursor.position else {
            return (cursor.clone(), cursor);
        };

        let new_node_ref = self.ordered_nodes.alloc(Node {
            kind: NodeKind::Branching(BranchNode {
                condition,
                when_true: None,
                when_false: None,
            }),
            source,
        });

        connect(&mut self.ordered_nodes, position, new_node_ref);

        (
            CursorPosition::new(new_node_ref, 0).into(),
            CursorPosition::new(new_node_ref, 1).into(),
        )
    }

    pub fn push_terminating(
        &mut self,
        cursor: Cursor,
        node: TerminatingNode,
        source: Source,
    ) -> Cursor {
        let Some(position) = cursor.position else {
            return cursor;
        };

        let new_node_ref = self.ordered_nodes.alloc(Node {
            kind: NodeKind::Terminating(node),
            source,
        });

        connect(&mut self.ordered_nodes, position, new_node_ref);
        Cursor::terminated()
    }

    #[must_use]
    pub fn push_scope(&mut self, cursor: Cursor, source: Source) -> (Cursor, Cursor) {
        let Some(position) = cursor.position else {
            return (cursor.clone(), cursor);
        };

        let new_node_ref = self.ordered_nodes.alloc(Node {
            kind: NodeKind::Scope(ScopeNode {
                inner: None,
                closed_at: None,
            }),
            source,
        });

        connect(&mut self.ordered_nodes, position, new_node_ref);

        (
            CursorPosition::new(new_node_ref, 0).into(),
            CursorPosition::new(new_node_ref, 1).into(),
        )
    }

    #[must_use]
    pub fn push_join(
        &mut self,
        cursor_a: Cursor,
        a_gives: Option<NodeRef>,
        cursor_b: Cursor,
        b_gives: Option<NodeRef>,
        conform_behavior: Option<ConformBehavior>,
        source: Source,
    ) -> Cursor {
        if (cursor_a.is_terminated() && cursor_b.is_terminated())
            || (a_gives.is_none() && b_gives.is_none())
        {
            return Cursor::terminated();
        }

        if let (Some(a_position), Some(b_position), Some(a_gives), Some(b_gives)) =
            (&cursor_a.position, &cursor_b.position, a_gives, b_gives)
        {
            let new_node_ref = self.ordered_nodes.alloc(Node {
                kind: NodeKind::Sequential(SequentialNode {
                    kind: SequentialNodeKind::JoinN(
                        smallvec![(a_position.clone(), a_gives), (b_position.clone(), b_gives)],
                        conform_behavior,
                    ),
                    next: None,
                }),
                source,
            });

            connect(&mut self.ordered_nodes, a_position.clone(), new_node_ref);
            connect(&mut self.ordered_nodes, b_position.clone(), new_node_ref);
            return CursorPosition::new(new_node_ref, 0).into();
        }

        if cursor_a.is_live() {
            if let Some(a_gives) = a_gives {
                return self.push_sequential(cursor_a, SequentialNodeKind::Join1(a_gives), source);
            }
        } else if cursor_b.is_live() {
            if let Some(b_gives) = b_gives {
                return self.push_sequential(cursor_b, SequentialNodeKind::Join1(b_gives), source);
            }
        }

        Cursor::terminated()
    }

    #[must_use]
    pub fn push_join_n(
        &mut self,
        incoming: impl IntoIterator<Item = (Cursor, Option<NodeRef>)>,
        conform_behavior: Option<ConformBehavior>,
        source: Source,
    ) -> Cursor {
        let incoming = incoming
            .into_iter()
            .filter_map(|(cursor, value)| cursor.position.zip(value))
            .collect::<SmallVec2<_>>();

        match incoming.len() {
            0 => Cursor::terminated(),
            1 => {
                let mut incoming = incoming.into_iter();
                let (position, gives_value) = incoming.next().unwrap();

                return self.push_sequential(
                    position.into(),
                    SequentialNodeKind::Join1(gives_value),
                    source,
                );
            }
            2 => {
                let mut incoming = incoming.into_iter();
                let (a_position, a_gives) = incoming.next().unwrap();
                let (b_position, b_gives) = incoming.next().unwrap();

                let new_node_ref = self.ordered_nodes.alloc(Node {
                    kind: NodeKind::Sequential(SequentialNode {
                        kind: SequentialNodeKind::JoinN(
                            smallvec![(a_position.clone(), a_gives), (b_position.clone(), b_gives)],
                            conform_behavior,
                        ),
                        next: None,
                    }),
                    source,
                });

                connect(&mut self.ordered_nodes, a_position, new_node_ref);
                connect(&mut self.ordered_nodes, b_position, new_node_ref);
                return CursorPosition::new(new_node_ref, 0).into();
            }
            _ => {
                let edges: Vec<_> = incoming
                    .iter()
                    .map(|(position, _)| position)
                    .cloned()
                    .collect();

                let new_node_ref = self.ordered_nodes.alloc(Node {
                    kind: NodeKind::Sequential(SequentialNode {
                        kind: SequentialNodeKind::JoinN(incoming, conform_behavior),
                        next: None,
                    }),
                    source,
                });

                for position in edges {
                    connect(&mut self.ordered_nodes, position, new_node_ref);
                }
                return CursorPosition::new(new_node_ref, 0).into();
            }
        }
    }

    #[must_use]
    pub fn const_eval(&mut self, expr: Expr) -> ConstEvalRef {
        let cfg = {
            let source = expr.source;
            let (mut builder, mut cursor) = Builder::new(self.const_evals, source);
            cursor = flatten_expr(&mut builder, cursor, expr, IsValue::RequireValue);
            let value = cursor.value();
            let _ = builder.push_terminating(cursor, TerminatingNode::Computed(value), source);
            builder.finish()
        };

        self.const_evals.alloc(ConstEval {
            context: HashMap::default(),
            cfg,
        })
    }

    #[must_use]
    pub fn finish(self) -> UntypedCfg {
        UntypedCfg {
            ordered_nodes: self.ordered_nodes,
            labels: self.labels,
        }
    }
}
