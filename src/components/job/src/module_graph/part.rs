use crate::{
    module_graph::{ModulePartHandle, ModuleRef, symbol_channel::SymbolChannel},
    repr::DeclHead,
};
use arena::{Idx, new_id_with_niche};
use attributes::Privacy;
use auto_enums::auto_enum;
use diagnostics::Show;
use std::{collections::HashMap, path::PathBuf};

new_id_with_niche!(ModulePartId, u32);
pub type ModulePartRef<'env> = Idx<ModulePartId, ModulePart<'env>>;

#[derive(Debug)]
pub struct ModulePart<'env> {
    private: SymbolChannel<'env>,
    hidden: Option<HiddenModulePartSymbols<'env>>,
}

#[derive(Copy, Clone, Debug)]
pub enum ModulePartVisibility {
    Visible,
    Hidden,
}

impl<'env> ModulePart<'env> {
    pub fn new(visibility: ModulePartVisibility) -> Self {
        Self {
            private: Default::default(),
            hidden: match visibility {
                ModulePartVisibility::Visible => None,
                ModulePartVisibility::Hidden => Some(Default::default()),
            },
        }
    }

    pub fn add_symbol(
        &mut self,
        privacy: Privacy,
        name: &'env str,
        decl_head: DeclHead<'env>,
    ) -> Result<(), ShouldBeAtModuleLevel> {
        if let Privacy::Private = privacy {
            self.private.add_symbol(name, decl_head);
            return Ok(());
        }

        let Some(hidden) = &mut self.hidden else {
            return Err(ShouldBeAtModuleLevel);
        };

        match privacy {
            Privacy::Public => &mut hidden.public,
            Privacy::Protected => &mut hidden.protected,
            Privacy::Private => unreachable!(),
        }
        .add_symbol(name, decl_head);

        Ok(())
    }

    #[auto_enum(Iterator)]
    pub fn iter_inner_symbols<'a>(
        &'a self,
        name: &'env str,
    ) -> impl Iterator<Item = DeclHead<'env>> {
        match &self.hidden {
            Some(hidden) => hidden
                .public
                .iter_symbols(name)
                .chain(hidden.protected.iter_symbols(name))
                .chain(self.private.iter_symbols(name)),
            None => self.private.iter_symbols(name),
        }
    }

    #[must_use]
    pub fn unhide(&mut self) -> Option<HiddenModulePartSymbols<'env>> {
        self.hidden.take()
    }

    pub fn private(&self) -> &SymbolChannel<'env> {
        &self.private
    }

    pub fn hidden(&self) -> Option<&HiddenModulePartSymbols<'env>> {
        self.hidden.as_ref()
    }
}

pub struct ShouldBeAtModuleLevel;

/// Hidden module parts are module parts that are exclusively
/// referenced by other module graphs.
/// For example,
/// If we add a part to a module only for the runtime target,
/// it still needs to reference the compile-time version of itself
/// in the compile-time module graph, despite the (compile-time version) module part
/// not being visible to other parts within the module (for the compile-time module graph).
#[derive(Debug, Default)]
pub struct HiddenModulePartSymbols<'env> {
    pub public: SymbolChannel<'env>,
    pub protected: SymbolChannel<'env>,
}
