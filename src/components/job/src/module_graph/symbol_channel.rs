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

    pub fn add_symbols(&mut self, batch: impl Iterator<Item = (&'env str, DeclHeadSet<'env>)>) {
        for (name, set) in batch {
            let existing_set = self.symbols.entry(name).or_default();

            for head in set.into_iter() {
                existing_set.push(head);
            }
        }
    }

    pub fn iter_symbols(&self, name: &'env str) -> impl Iterator<Item = DeclHead<'env>> {
        self.symbols
            .get(name)
            .into_iter()
            .flat_map(|head| head.iter())
    }

    pub fn into_symbols(self) -> impl Iterator<Item = (&'env str, DeclHeadSet<'env>)> {
        self.symbols.into_iter()
    }

    pub fn symbols(&self) -> &HashMap<&'env str, DeclHeadSet<'env>> {
        &self.symbols
    }
}
