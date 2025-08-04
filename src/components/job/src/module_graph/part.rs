use crate::{module_graph::symbol_channel::SymbolChannel, repr::DeclHead};
use arena::{Idx, new_id_with_niche};

new_id_with_niche!(ModulePartId, u32);
pub type ModulePartRef<'env> = Idx<ModulePartId, ModulePart<'env>>;

#[derive(Debug, Default)]
pub struct ModulePart<'env> {
    private: SymbolChannel<'env>,
}

impl<'env> ModulePart<'env> {
    pub fn add_symbol(&mut self, name: &'env str, decl_head: DeclHead<'env>) {
        self.private.add_symbol(name, decl_head);
    }

    pub fn private(&self) -> &SymbolChannel<'env> {
        &self.private
    }
}
