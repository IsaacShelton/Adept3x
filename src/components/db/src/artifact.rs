use crate::NumberedRevision;
use std::collections::HashMap;

// The result of each value is predicated on some tree
// of assumptions that were made in the process of
// computing this value.
// If any of these assumptions are later invalidated,
// we can snip off any bad requests caused by those
// incorrect assumptions.
#[derive(Clone, Debug)]
pub enum Artifact {
    Void,
    Bool(bool),
    String(String),
    Found(Vec<SymbolId>),
}

#[derive(Clone, Debug)]
pub struct SymbolId(u32);
