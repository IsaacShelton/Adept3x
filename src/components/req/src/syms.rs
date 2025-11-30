use crate::Pf;
use std::{
    cmp::max,
    collections::{HashMap, HashSet},
};

#[derive(Clone, Debug, Default)]
pub struct Syms<P: Pf> {
    pub named: HashMap<String, SymGrp<P>>,
    pub evals: HashMap<Eval, P::Rev>,
}

impl<P: Pf> PartialEq for Syms<P> {
    fn eq(&self, other: &Self) -> bool {
        !self.has_changed(other)
    }
}

impl<P: Pf> Eq for Syms<P> {}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct Eval(String);

impl<P: Pf> Syms<P> {
    pub fn has_changed(&self, other: &Syms<P>) -> bool {
        if self.named.len() != other.named.len() {
            return true;
        }

        if self.evals.len() != other.evals.len() {
            return true;
        }

        for (self_name, self_grp) in &self.named {
            let Some(other_grp) = other.named.get(self_name) else {
                return true;
            };

            if self_grp.has_changed(&other_grp) {
                return true;
            }
        }

        for (self_eval, _) in &self.evals {
            if other.evals.get(&self_eval).is_none() {
                return true;
            }
        }

        false
    }
}

#[derive(Clone, Debug)]
pub struct SymGrp<P: Pf> {
    syms: HashSet<Sym>,
    last_changed: P::Rev,
}

impl<P: Pf> SymGrp<P> {
    pub fn insert(&mut self, sym: Sym, rev: P::Rev) {
        self.syms.insert(sym);
        self.last_changed = max(self.last_changed, rev);
    }

    pub fn last_changed(&self) -> P::Rev {
        self.last_changed
    }

    pub fn has_changed(&self, other: &SymGrp<P>) -> bool {
        if self.syms.len() != other.syms.len() {
            return true;
        }

        for sym in self.syms.iter() {
            if !other.syms.contains(&sym) {
                return true;
            }
        }

        false
    }

    pub fn iter(&self) -> impl Iterator<Item = &Sym> {
        self.syms.iter()
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct SymId(usize);

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct Sym {
    kind: SymKind,
    id: SymId,
    from_eval: Option<Eval>,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum SymKind {
    Func(Func),
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct Func;
