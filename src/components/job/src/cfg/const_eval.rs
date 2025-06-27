use super::UntypedCfg;
use arena::{Idx, new_id_with_niche};
use source_files::Source;
use std::collections::HashMap;

new_id_with_niche!(ConstEvalId, u64);

pub type ConstEvalRef = Idx<ConstEvalId, ConstEval>;

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct ConstEval {
    pub context: HashMap<String, Vec<SymbolBinding>>,
    pub cfg: UntypedCfg,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct SymbolBinding {
    pub symbol: SymbolRef,
    pub source: Source,
}

pub type SymbolRef = ConstEvalRef;
