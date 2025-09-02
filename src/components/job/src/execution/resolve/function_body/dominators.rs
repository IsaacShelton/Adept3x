use crate::{BasicBlockId, CfgBuilder, EndInstrKind};
use arena::{ArenaMap, Id};
use bit_vec::BitVec;
use std::{
    cmp::Ordering,
    collections::{HashSet, VecDeque},
};

pub type PostOrder = Box<[BasicBlockId]>;

pub fn compute_idom_tree(cfg: &CfgBuilder) -> (ArenaMap<BasicBlockId, BasicBlockId>, PostOrder) {
    let start = cfg.start();
    let (post_order, post_order_map, pred_sets) = depth_first_search(cfg);

    let mut dominators = Dominators::with_capacity(cfg.len());

    let root = *post_order.last().unwrap();
    dominators.insert(root, root);

    let mut changed = true;
    while changed {
        changed = false;

        for bb_id in post_order.iter().rev().skip(1).copied() {
            debug_assert!(bb_id != start);

            let mut completed_predecessors = pred_sets
                .get(bb_id)
                .unwrap()
                .iter()
                .copied()
                .filter(|predecessor| dominators.get(*predecessor).is_some());

            let first_completed_predecessor = completed_predecessors
                .next()
                .expect("There must exist a predecessor that is also a dominator");

            let new_idom = completed_predecessors.fold(
                first_completed_predecessor,
                |new_idom, predecessor| {
                    intersect(&dominators, &post_order_map, new_idom, predecessor)
                },
            );

            if Some(new_idom) != dominators.get(bb_id).copied() {
                dominators.insert(bb_id, new_idom);
                changed = true;
            }
        }
    }

    (dominators, post_order.into())
}

fn intersect(
    dominators: &Dominators,
    post_order_map: &PostOrderIndexMap,
    mut a: BasicBlockId,
    mut b: BasicBlockId,
) -> BasicBlockId {
    let mut a_idx = *post_order_map.get(a).unwrap();
    let mut b_idx = *post_order_map.get(b).unwrap();

    loop {
        match a_idx.cmp(&b_idx) {
            Ordering::Less => {
                a = *dominators.get(a).unwrap();
                a_idx = *post_order_map.get(a).unwrap();
            }
            Ordering::Equal => return a,
            Ordering::Greater => {
                b = *dominators.get(b).unwrap();
                b_idx = *post_order_map.get(b).unwrap();
            }
        }
    }
}

type Dominators = ArenaMap<BasicBlockId, BasicBlockId>;
type PostOrderIndex = u32;
type PostOrderIndexMap = ArenaMap<BasicBlockId, PostOrderIndex>;
type PredecessorSets = ArenaMap<BasicBlockId, HashSet<BasicBlockId>>;

fn depth_first_search(cfg: &CfgBuilder) -> (Vec<BasicBlockId>, PostOrderIndexMap, PredecessorSets) {
    let start = cfg.start();

    let mut post_order = Vec::with_capacity(cfg.len());
    let mut post_order_map = ArenaMap::with_capacity(cfg.len());
    let mut visited = BitVec::from_elem(cfg.len(), false);
    let mut queue = VecDeque::with_capacity(64);
    let mut predecessor_sets = PredecessorSets::new();

    queue.push_back(start);

    loop {
        let Some(bb_id) = queue.front().copied() else {
            break;
        };

        let bb = cfg.get_unsafe(bb_id);

        let mut enqueue = |next: BasicBlockId| {
            let index = next.into_usize();

            // Track predecessors
            {
                let next_id = next;
                if predecessor_sets.get(next_id).is_none() {
                    predecessor_sets.insert(next_id, HashSet::new());
                }
                predecessor_sets.get_mut(next_id).unwrap().insert(bb_id);
            }

            if !visited.get(index).unwrap() {
                queue.push_front(next);
                visited.set(index, true);
                true
            } else {
                false
            }
        };

        let pending = match bb.end.as_ref().unwrap().kind {
            EndInstrKind::IncompleteGoto(_) => panic!("cannot dfs basicblock with incomplete goto"),
            EndInstrKind::IncompleteBreak => panic!("cannot dfs basicblock with incomplete break"),
            EndInstrKind::IncompleteContinue => {
                panic!("cannot dfs basicblock with incomplete continue")
            }
            EndInstrKind::Return(_, _) => false,
            EndInstrKind::Jump(next, _) => enqueue(next),
            EndInstrKind::Branch(_, when_true, when_false, _) => {
                enqueue(when_true) || enqueue(when_false)
            }
            EndInstrKind::NewScope(in_scope, close_scope) => {
                enqueue(in_scope) || enqueue(close_scope)
            }
            EndInstrKind::Unreachable => false,
        };

        if !pending {
            queue.pop_front();
            let number = post_order.len().try_into().unwrap();
            post_order_map.insert(bb_id, number);
            post_order.push(bb_id);
        }
    }

    return (post_order, post_order_map, predecessor_sets);
}
