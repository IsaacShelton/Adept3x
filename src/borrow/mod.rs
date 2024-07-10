/*
    Experimental borrow checker for evaluating the potential
    of fast and simple origin-based borrow checking.

    If profitable, then it could be used as the underpinning
    of the language.

    It would be an opt-in abstraction, and people wouldn't need
    to understand borrow checking in order to benefit from it
    being used in 3rd party libraries when using the simpler
    arc/rc-style types.

    (e.g. the atomically-reference-counted String type would seemlessly
    interop with functions that take string data by immutable reference)

    It would also be faster, simpler, and more versitile than
    Rust's borrow checker in most cases. The only thing unsupported
    would be reborrowing in the same scope with implicit lifetime. Other
    non-lexical-lifetime-like features would work however, and we would
    support returning partial references (unlike Rust).

    It would have an approximate average time complexity of O(n*r*o),
    and if we limit the maximum number of nested loops, then it
    would become proper O(n*r*o) worst case, where (per function)
      n = # of instructions
      r = # of variables/temporaries of reference types
      o = # of origins (items that can be separately borrowed)
*/

use bitvec::{bitbox, boxed::BitBox};
use derive_more::IsVariant;
use itertools::Itertools;

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ImmutableReferrerIdx(usize);

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct MutableReferrerIdx(usize);

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct OriginIdx(usize);

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct OriginRange {
    pub start: OriginIdx,
    pub end_exclusive: OriginIdx,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReferrerSet {
    is_referenced_by: BitBox,
}

impl ReferrerSet {
    pub fn new(num_referrers: usize) -> Self {
        Self {
            is_referenced_by: bitbox![0; num_referrers],
        }
    }

    pub fn capacity(&self) -> usize {
        self.is_referenced_by.len()
    }

    pub fn is_empty(&self) -> bool {
        self.is_referenced_by.iter_ones().next().is_none()
    }

    pub fn at_least_one(&self) -> Option<usize> {
        self.is_referenced_by.iter_ones().next()
    }

    pub fn iter(&self) -> impl Iterator<Item = usize> + '_ {
        self.is_referenced_by.iter_ones()
    }

    pub fn borrow(&mut self, index: usize) {
        self.is_referenced_by.set(index, true);
    }

    pub fn unborrow(&mut self, index: usize) {
        self.is_referenced_by.set(index, false);
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, IsVariant)]
pub enum OriginState {
    Owned,
    Moved,
    Dead,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum GenericReferrerIdx {
    Mutable(MutableReferrerIdx),
    Immutable(ImmutableReferrerIdx),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Origin {
    state: OriginState,
    potential_immutable_referrers: ReferrerSet,
    potential_mutable_referrers: ReferrerSet,
    is_reference: Option<GenericReferrerIdx>,
}

impl Origin {
    pub fn new(
        num_immutable_referrers: usize,
        num_mutable_referrers: usize,
        is_reference: Option<GenericReferrerIdx>,
    ) -> Self {
        Self {
            state: OriginState::Dead,
            potential_immutable_referrers: ReferrerSet::new(num_immutable_referrers),
            potential_mutable_referrers: ReferrerSet::new(num_mutable_referrers),
            is_reference,
        }
    }

    pub fn move_out(&mut self) -> Result<(), ()> {
        if self.state != OriginState::Owned
            || !self.potential_mutable_referrers.is_empty()
            || !self.potential_immutable_referrers.is_empty()
        {
            return Err(());
        }

        self.state = OriginState::Moved;
        Ok(())
    }

    pub fn is_borrowed(&self) -> bool {
        self.is_mutably_borrowed().is_some() || self.is_immutably_borrowed().is_some()
    }

    pub fn is_mutably_borrowed(&self) -> Option<MutableReferrerIdx> {
        self.potential_mutable_referrers
            .at_least_one()
            .map(MutableReferrerIdx)
    }

    pub fn is_immutably_borrowed(&self) -> Option<ImmutableReferrerIdx> {
        self.potential_immutable_referrers
            .at_least_one()
            .map(ImmutableReferrerIdx)
    }

    pub fn borrow_mutable(&mut self, referrer: MutableReferrerIdx) -> Result<(), ()> {
        if self.is_mutably_borrowed().is_none() && self.is_immutably_borrowed().is_none() {
            self.potential_mutable_referrers.borrow(referrer.0);
            Ok(())
        } else {
            Err(())
        }
    }

    pub fn borrow_immutable(&mut self, referrer: ImmutableReferrerIdx) -> Result<(), ()> {
        if self.is_mutably_borrowed().is_none() {
            self.potential_immutable_referrers.borrow(referrer.0);
            Ok(())
        } else {
            Err(())
        }
    }

    pub fn unborrow(&mut self, referrer: GenericReferrerIdx) {
        match referrer {
            GenericReferrerIdx::Immutable(referrer) => self.unborrow_immutable(referrer),
            GenericReferrerIdx::Mutable(referrer) => self.unborrow_mutable(referrer),
        }
    }

    pub fn unborrow_mutable(&mut self, referrer: MutableReferrerIdx) {
        self.potential_mutable_referrers.unborrow(referrer.0);
    }

    pub fn unborrow_immutable(&mut self, referrer: ImmutableReferrerIdx) {
        self.potential_immutable_referrers.unborrow(referrer.0);
    }

    pub fn incorporate(&mut self, other: &Self) {
        self.potential_mutable_referrers.is_referenced_by &= other
            .potential_mutable_referrers
            .is_referenced_by
            .as_bitslice();

        self.potential_immutable_referrers.is_referenced_by &= other
            .potential_immutable_referrers
            .is_referenced_by
            .as_bitslice();
    }

    pub fn immutable_referrers_capacity(&self) -> usize {
        self.potential_immutable_referrers.capacity()
    }

    pub fn mutable_referrers_capacity(&self) -> usize {
        self.potential_immutable_referrers.capacity()
    }
}

#[derive(Clone, Debug)]
pub struct Referrer {
    name: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Point {
    origins: Box<[Origin]>,
}

pub enum OriginTemplate {
    Regular,
    AlsoReference(GenericReferrerIdx),
}

// Represents an origin that should be dropped
// for an incoming edge. This happens when some of
// the incoming edges have moved an origin, but
// some still have it sticking around.
#[derive(Clone, Debug)]
pub struct JoinDropSideEffect {
    pub incoming_edge: usize,
    pub need_to_drop: OriginIdx,
}

#[derive(Clone, Debug)]
pub struct JoinPoint {
    pub joined_point: Point,
    pub side_effects: Vec<JoinDropSideEffect>,
}

// The origin is still being borrowed even after it just died.
// This means that the origin doesn't live long enough, and
// anyone still borrowing it shouldn't have been able to borrow it.
pub struct StillBorrowedAfterDeath;

impl Point {
    pub fn new(
        origin_templates: &[OriginTemplate],
        num_immutable_referrers: usize,
        num_mutable_referrers: usize,
    ) -> Self {
        let origins = origin_templates
            .iter()
            .map(|template| {
                Origin::new(
                    num_immutable_referrers,
                    num_mutable_referrers,
                    match template {
                        OriginTemplate::Regular => None,
                        OriginTemplate::AlsoReference(as_reference) => Some(*as_reference),
                    },
                )
            })
            .collect_vec()
            .into_boxed_slice();

        Self { origins }
    }

    pub fn birth_origin(&mut self, origin: OriginIdx) {
        let origin = &mut self.origins[origin.0];
        assert_eq!(origin.state, OriginState::Dead);
        assert!(origin.potential_mutable_referrers.is_empty());
        assert!(origin.potential_immutable_referrers.is_empty());
        origin.state = OriginState::Owned;
    }

    pub fn start_origin_death(&mut self, origin: OriginIdx) {
        let origin = &self.origins[origin.0];

        if origin.state == OriginState::Owned {
            if let Some(dropped_reference) = origin.is_reference {
                for referenced in self.origins.iter_mut() {
                    referenced.unborrow(dropped_reference);
                }
            }
        }
    }

    pub fn finalize_origin_death(
        &mut self,
        origin: OriginIdx,
    ) -> Result<(), StillBorrowedAfterDeath> {
        let origin = &mut self.origins[origin.0];

        match origin.state {
            OriginState::Owned => {
                if origin.is_borrowed() {
                    Err(StillBorrowedAfterDeath)
                } else {
                    origin.state = OriginState::Dead;
                    Ok(())
                }
            }
            OriginState::Moved => {
                origin.state = OriginState::Dead;
                Ok(())
            }
            OriginState::Dead => {
                panic!("variable is already dead?");
            }
        }
    }

    // NOTE: Borrows made in scope should be unborrowed before
    // joining control flow. This function will give back the
    // resulting borrow state from joining several points in time
    // together. There will also be a list of side effects returned
    // that specify which owned origins should be dropped before the join.
    // (These side effects occur due to some incoming branches having
    // moved their origin values while others haven't)
    pub fn join(points: &[Self]) -> JoinPoint {
        if points.len() == 0 {
            return JoinPoint {
                joined_point: Point {
                    origins: Box::new([]),
                },
                side_effects: vec![],
            };
        }

        let mut origins = points[0]
            .origins
            .iter()
            .map(|existing_origin| {
                Origin::new(
                    existing_origin.immutable_referrers_capacity(),
                    existing_origin.mutable_referrers_capacity(),
                    existing_origin.is_reference,
                )
            })
            .collect_vec();

        let expected_origin_count = origins.len();
        let mut side_effects = vec![];

        for (origin_idx, result_origin) in origins.iter_mut().enumerate() {
            let mut joined_state = OriginState::Dead;
            let mut join_state_conflict = false;

            for incoming_point in points.iter() {
                let incoming_point_origins = &incoming_point.origins;
                assert_eq!(expected_origin_count, incoming_point_origins.len());

                let incoming_origin = &incoming_point_origins[origin_idx];
                result_origin.incorporate(incoming_origin);

                if joined_state.is_dead() {
                    assert!(!incoming_origin.state.is_dead());
                    joined_state = incoming_origin.state;
                } else if incoming_origin.state != joined_state {
                    joined_state = OriginState::Moved;
                    join_state_conflict = true;
                }
            }

            // Resolve join state conflict
            if join_state_conflict {
                for (point_i, incoming_point) in points.iter().enumerate() {
                    let incoming_point_origin = &incoming_point.origins[origin_idx];

                    if incoming_point_origin.state.is_owned() {
                        side_effects.push(JoinDropSideEffect {
                            incoming_edge: point_i,
                            need_to_drop: OriginIdx(origin_idx),
                        });
                    }
                }
            }
        }

        let origins = origins.into_boxed_slice();

        JoinPoint {
            joined_point: Point { origins },
            side_effects,
        }
    }

    pub fn borrow_immutable(
        &mut self,
        origin: OriginIdx,
        referrer: ImmutableReferrerIdx,
    ) -> Result<(), ()> {
        self.origins[origin.0].borrow_immutable(referrer)
    }

    pub fn borrow_mutable(
        &mut self,
        origin: OriginIdx,
        referrer: MutableReferrerIdx,
    ) -> Result<(), ()> {
        self.origins[origin.0].borrow_mutable(referrer)
    }

    pub fn borrow_immutable_range(
        &mut self,
        origin_range: OriginRange,
        referrer: ImmutableReferrerIdx,
    ) -> Result<(), ()> {
        for index in origin_range.start.0..origin_range.end_exclusive.0 {
            self.origins[index].borrow_immutable(referrer)?
        }
        Ok(())
    }

    pub fn borrow_mutable_range(
        &mut self,
        origin_range: OriginRange,
        referrer: MutableReferrerIdx,
    ) -> Result<(), ()> {
        for index in origin_range.start.0..origin_range.end_exclusive.0 {
            self.origins[index].borrow_mutable(referrer)?
        }
        Ok(())
    }
}
