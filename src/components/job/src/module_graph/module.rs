use crate::{
    module_graph::{
        FoundOrCreated, HiddenModulePartSymbols, LookupError, ModulePart, ModulePartHandle,
        ModulePartId, ModulePartVisibility, ShouldBeAtModuleLevel, part::ModulePartRef,
        symbol_channel::SymbolChannel,
    },
    repr::{DeclHead, DeclHeadSet},
};
use arena::{Arena, LockFreeArena};
use attributes::Privacy;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

#[derive(Debug, Default)]
pub struct Module<'env> {
    public: SymbolChannel<'env>,
    protected: SymbolChannel<'env>,
    parts: Arena<ModulePartId, ModulePart<'env>>,
    filenames: HashMap<PathBuf, ModulePartRef<'env>>,
}

impl<'env> Module<'env> {
    pub fn find_or_create_part(
        &mut self,
        canonical_filename: &Path,
        visibility: ModulePartVisibility,
    ) -> FoundOrCreated<ModulePartRef<'env>> {
        if let Some(found) = self.filenames.get(canonical_filename) {
            return FoundOrCreated::Found(*found);
        }

        let part = self.parts.alloc(ModulePart::new(visibility));
        self.filenames.insert(canonical_filename.into(), part);
        FoundOrCreated::Created(part)
    }

    pub fn get(&self, part_ref: ModulePartRef<'env>) -> &ModulePart<'env> {
        &self.parts[part_ref]
    }

    pub fn get_mut(&mut self, part_ref: ModulePartRef<'env>) -> &mut ModulePart<'env> {
        &mut self.parts[part_ref]
    }

    pub fn add_previously_hidden(&mut self, hidden: HiddenModulePartSymbols<'env>) {
        self.public.add_symbols(hidden.public.into_symbols());
        self.protected.add_symbols(hidden.protected.into_symbols());
    }

    pub fn add_symbol(
        &mut self,
        part_ref: ModulePartRef<'env>,
        privacy: Privacy,
        name: &'env str,
        decl_head: DeclHead<'env>,
    ) {
        let part = &mut self.parts[part_ref];

        match part.add_symbol(privacy, name, decl_head) {
            Ok(()) => (),
            Err(ShouldBeAtModuleLevel) => match privacy {
                Privacy::Public => self.public.add_symbol(name, decl_head),
                Privacy::Protected => self.protected.add_symbol(name, decl_head),
                Privacy::Private => unreachable!(),
            },
        }
    }

    pub fn iter_symbols(
        &self,
        name: &'env str,
        part_ref: ModulePartRef<'env>,
    ) -> impl Iterator<Item = &DeclHead<'env>> {
        let public = &self.public;
        let protected = &self.protected;

        self.parts[part_ref]
            .iter_symbols(name)
            .chain(public.iter_symbols(name))
            .chain(protected.iter_symbols(name))
    }

    pub fn iter_public_symbols(&self, name: &'env str) -> impl Iterator<Item = &DeclHead<'env>> {
        self.public.iter_symbols(name)
    }

    pub fn public(&self) -> &SymbolChannel<'env> {
        &self.public
    }

    pub fn protected(&self) -> &SymbolChannel<'env> {
        &self.protected
    }

    pub fn parts(&self) -> &Arena<ModulePartId, ModulePart<'env>> {
        &self.parts
    }
}
