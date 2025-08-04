use crate::repr::{DeclHead, DeclHeadSet};
use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct SymbolChannel<'env> {
    symbols: HashMap<&'env str, DeclHeadSet<'env>>,
}

impl<'env> SymbolChannel<'env> {
    pub fn add_symbol(&mut self, name: &'env str, decl_head: DeclHead<'env>) {
        self.symbols.entry(name).or_default().push(decl_head);
    }

    pub fn iter_symbols(&self, name: &'env str) -> impl Iterator<Item = &DeclHead<'env>> {
        self.symbols
            .get(name)
            .into_iter()
            .flat_map(|head| head.iter())
    }
}
