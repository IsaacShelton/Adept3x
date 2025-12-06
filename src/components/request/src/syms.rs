use crate::{Pf, TopErrors};
use serde::{Deserialize, Serialize};
use std::{
    cmp::max,
    collections::{HashMap, HashSet},
};

#[derive(Clone, Debug, Default, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct WithErrors<T> {
    pub value: T,
    pub errors: TopErrors,
}

impl<T> WithErrors<T> {
    pub fn new(value: T, errors: TopErrors) -> Self {
        Self { value, errors }
    }

    pub fn no_errors(value: T) -> Self {
        Self {
            value,
            errors: TopErrors::default(),
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
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

#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Clone, Debug, Serialize, Deserialize)]
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

#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct SymId(usize);

#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sym {
    kind: SymKind,
    id: SymId,
    from_eval: Option<Eval>,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum SymKind {
    Func(Func),
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct Func;
