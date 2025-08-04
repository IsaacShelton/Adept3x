use crate::{
    module_graph::{
        LookupError, ModulePart, ModulePartId, part::ModulePartRef, symbol_channel::SymbolChannel,
    },
    repr::{DeclHead, DeclHeadSet},
};
use arena::{Arena, LockFreeArena};
use attributes::Privacy;

#[derive(Debug, Default)]
pub struct Module<'env> {
    public: SymbolChannel<'env>,
    protected: SymbolChannel<'env>,
    parts: Arena<ModulePartId, ModulePart<'env>>,
}

impl<'env> Module<'env> {
    pub fn add_part(&mut self) -> ModulePartRef<'env> {
        self.parts.alloc(ModulePart::default())
    }

    pub fn get(&self, part_ref: ModulePartRef<'env>) -> &ModulePart<'env> {
        &self.parts[part_ref]
    }

    pub fn add_symbol(
        &mut self,
        privacy: Privacy,
        name: &'env str,
        decl_head: DeclHead<'env>,
        part_ref: ModulePartRef<'env>,
    ) {
        match privacy {
            Privacy::Public => self.public.add_symbol(name, decl_head),
            Privacy::Protected => self.protected.add_symbol(name, decl_head),
            Privacy::Private => self.parts[part_ref].add_symbol(name, decl_head),
        }
    }

    pub fn iter_symbols(
        &self,
        name: &'env str,
        part_ref: ModulePartRef<'env>,
    ) -> impl Iterator<Item = &DeclHead<'env>> {
        let public = &self.public;
        let protected = &self.protected;
        let private = &self.parts[part_ref].private();

        public
            .iter_symbols(name)
            .chain(protected.iter_symbols(name))
            .chain(private.iter_symbols(name))
    }

    pub fn iter_public_symbols(&self, name: &'env str) -> impl Iterator<Item = &DeclHead<'env>> {
        self.public.iter_symbols(name)
    }
}
