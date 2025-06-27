use crate::cfg::{NodeId, NodeKind, NodeRef, UntypedCfg};
use arena::{ArenaMap, Id};
use bit_vec::BitVec;
use std::{
    cmp::Ordering,
    collections::{HashSet, VecDeque},
};

pub type PostOrder = Vec<NodeRef>;

pub fn compute_idom_tree(cfg: &UntypedCfg) -> (ArenaMap<NodeId, NodeRef>, PostOrder) {
    let start = cfg.start();
    let (post_order, post_order_map, pred_sets) = depth_first_search(cfg);

    let mut dominators = Dominators::with_capacity(cfg.len());

    let root = post_order.last().unwrap();
    dominators.insert(root.into_raw(), *root);

    let mut changed = true;
    while changed {
        changed = false;

        for node_ref in post_order.iter().rev().skip(1).copied() {
            debug_assert!(node_ref != start);

            let mut completed_predecessors = pred_sets
                .get(node_ref.into_raw())
                .unwrap()
                .iter()
                .filter(|predecessor| dominators.get(predecessor.into_raw()).is_some());

            let first_completed_predecessor = *completed_predecessors
                .next()
                .expect("There must exist a predecessor that is also a dominator");

            let new_idom = completed_predecessors.fold(
                first_completed_predecessor,
                |new_idom, predecessor| {
                    intersect(&dominators, &post_order_map, new_idom, *predecessor)
                },
            );

            if Some(new_idom) != dominators.get(node_ref.into_raw()).copied() {
                dominators.insert(node_ref.into_raw(), new_idom);
                changed = true;
            }
        }
    }

    (dominators, post_order)
}

fn intersect(
    dominators: &Dominators,
    post_order_map: &PostOrderIndexMap,
    mut a: NodeRef,
    mut b: NodeRef,
) -> NodeRef {
    let mut a_idx = *post_order_map.get(a.into_raw()).unwrap();
    let mut b_idx = *post_order_map.get(b.into_raw()).unwrap();

    loop {
        match a_idx.cmp(&b_idx) {
            Ordering::Less => {
                a = *dominators.get(a.into_raw()).unwrap();
                a_idx = *post_order_map.get(a.into_raw()).unwrap();
            }
            Ordering::Equal => return a,
            Ordering::Greater => {
                b = *dominators.get(b.into_raw()).unwrap();
                b_idx = *post_order_map.get(b.into_raw()).unwrap();
            }
        }
    }
}

type Dominators = ArenaMap<NodeId, NodeRef>;
type PostOrderIndex = usize;
type PostOrderIndexMap = ArenaMap<NodeId, PostOrderIndex>;
type PredecessorSets = ArenaMap<NodeId, HashSet<NodeRef>>;

fn depth_first_search(cfg: &UntypedCfg) -> (Vec<NodeRef>, PostOrderIndexMap, PredecessorSets) {
    let start = cfg.start();

    let mut post_order = Vec::with_capacity(cfg.len());
    let mut post_order_map = ArenaMap::with_capacity(cfg.len());
    let mut visited = BitVec::from_elem(cfg.len(), false);
    let mut queue = VecDeque::with_capacity(64);
    let mut predecessor_sets = PredecessorSets::new();

    queue.push_back(start);

    loop {
        let Some(node_ref) = queue.front().copied() else {
            break;
        };

        let node = &cfg.ordered_nodes[node_ref];

        let mut enqueue = |next: Option<NodeRef>| {
            if let Some(next) = next {
                let index = next.into_raw().into_usize();

                // Track predecessors
                {
                    let next_id = next.into_raw();
                    if predecessor_sets.get(next_id).is_none() {
                        predecessor_sets.insert(next_id, HashSet::new());
                    }
                    predecessor_sets.get_mut(next_id).unwrap().insert(node_ref);
                }

                if !visited.get(index).unwrap() {
                    queue.push_front(next);
                    visited.set(index, true);
                    true
                } else {
                    false
                }
            } else {
                false
            }
        };

        let pending = match &node.kind {
            NodeKind::Start(next) => enqueue(*next),
            NodeKind::Sequential(seq) => enqueue(seq.next),
            NodeKind::Branching(branch) => enqueue(branch.when_true) || enqueue(branch.when_false),
            NodeKind::Terminating(_) => false,
            NodeKind::Scope(scope) => enqueue(scope.inner) || enqueue(scope.closed_at),
        };

        if !pending {
            queue.pop_front();
            let number = post_order.len();
            post_order_map.insert(node_ref.into_raw(), number);
            post_order.push(node_ref);
        }
    }

    return (post_order, post_order_map, predecessor_sets);
}
