use crate::{module_graph::symbol_channel::SymbolChannel, repr::DeclHead};
use arena::{Idx, new_id_with_niche};

new_id_with_niche!(ModulePartId, u32);
pub type ModulePartRef<'env> = Idx<ModulePartId, ModulePart<'env>>;

#[derive(Debug, Default)]
pub struct ModulePart<'env> {
    private: SymbolChannel<'env>,
    hidden: Option<HiddenModulePartSymbols<'env>>,
}

impl<'env> ModulePart<'env> {
    pub fn add_symbol(&mut self, name: &'env str, decl_head: DeclHead<'env>) {
        self.private.add_symbol(name, decl_head);
    }

    pub fn private(&self) -> &SymbolChannel<'env> {
        &self.private
    }
}

/// Hidden module parts are module parts that are exclusively
/// referenced by other workspaces.
/// For example,
/// If we add a part to a module only for the runtime target,
/// it still needs to reference the compile-time version of itself
/// in the compile-time workspace, despite the (compile-time version) module part
/// not being visible to other parts within the module (for the compile-time workspace).
#[derive(Debug, Default)]
pub struct HiddenModulePartSymbols<'env> {
    public: SymbolChannel<'env>,
    protected: SymbolChannel<'env>,
}
