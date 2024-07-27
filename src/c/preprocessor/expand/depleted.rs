use crate::c::preprocessor::ast::Define;
use std::{
    collections::HashSet,
    hash::{DefaultHasher, Hash, Hasher},
};

/*
    Stack of definitions that have already been used in a given expansion path.

    Whenever a #define is used, it is checked against this list to see if it's already been used.
    - If the #define has already been used, it should be left alone according to the standard.
    - Otherwise, it will be substituted for its definition.

    NOTE: We store only the hashes of the `#define`s already used in this expansion path.
    This should improve performance, reduce memory consumption, and since we're using a
    cryptographic hash, collisions should be effectively impossible.

    Also, since we store the hash directly, we get to skip recomputing it two extra times
    for each expansion of a `#define`.
*/

pub struct Depleted {
    pub hashes: HashSet<u64>,
}

impl Depleted {
    pub fn new() -> Self {
        Self {
            hashes: Default::default(),
        }
    }

    pub fn push(&mut self, hash: u64) {
        self.hashes.insert(hash);
    }

    pub fn pop(&mut self, hash: u64) {
        self.hashes.remove(&hash);
    }

    pub fn contains(&self, hash: u64) -> bool {
        self.hashes.contains(&hash)
    }

    pub fn hash_define(define: &Define) -> u64 {
        let mut hasher = DefaultHasher::new();
        define.name.hash(&mut hasher);
        hasher.finish()
    }
}
