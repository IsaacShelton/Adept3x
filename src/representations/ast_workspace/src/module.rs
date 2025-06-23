use crate::{
    AstFile, AstWorkspaceSymbols, EnumRef, ExprAliasRef, FuncRef, GlobalRef, ImplRef, NameScopeRef,
    NamespaceRef, StructRef, TraitRef, TypeAliasRef,
};

#[derive(Debug)]
pub struct Module {
    pub files: Vec<AstFile>,
}

impl Module {
    pub fn name_scopes(&self) -> impl Iterator<Item = NameScopeRef> {
        self.files.iter().map(|file| file.names)
    }

    pub fn funcs(&self, symbols: &AstWorkspaceSymbols) -> impl Iterator<Item = FuncRef> {
        self.files
            .iter()
            .flat_map(|file| symbols.all_name_scopes[file.names].funcs.iter())
    }

    pub fn structs(&self, symbols: &AstWorkspaceSymbols) -> impl Iterator<Item = StructRef> {
        self.files
            .iter()
            .flat_map(|file| symbols.all_name_scopes[file.names].structs.iter())
    }

    pub fn enums(&self, symbols: &AstWorkspaceSymbols) -> impl Iterator<Item = EnumRef> {
        self.files
            .iter()
            .flat_map(|file| symbols.all_name_scopes[file.names].enums.iter())
    }

    pub fn globals(&self, symbols: &AstWorkspaceSymbols) -> impl Iterator<Item = GlobalRef> {
        self.files
            .iter()
            .flat_map(|file| symbols.all_name_scopes[file.names].globals.iter())
    }

    pub fn type_aliases(
        &self,
        symbols: &AstWorkspaceSymbols,
    ) -> impl Iterator<Item = TypeAliasRef> {
        self.files
            .iter()
            .flat_map(|file| symbols.all_name_scopes[file.names].type_aliases.iter())
    }

    pub fn expr_aliases(
        &self,
        symbols: &AstWorkspaceSymbols,
    ) -> impl Iterator<Item = ExprAliasRef> {
        self.files
            .iter()
            .flat_map(|file| symbols.all_name_scopes[file.names].expr_aliases.iter())
    }

    pub fn traits(&self, symbols: &AstWorkspaceSymbols) -> impl Iterator<Item = TraitRef> {
        self.files
            .iter()
            .flat_map(|file| symbols.all_name_scopes[file.names].traits.iter())
    }

    pub fn impls(&self, symbols: &AstWorkspaceSymbols) -> impl Iterator<Item = ImplRef> {
        self.files
            .iter()
            .flat_map(|file| symbols.all_name_scopes[file.names].impls.iter())
    }

    pub fn namespaces<'a>(
        &'a self,
        symbols: &'a AstWorkspaceSymbols,
    ) -> impl Iterator<Item = NamespaceRef> {
        self.files
            .iter()
            .flat_map(|file| symbols.all_name_scopes[file.names].namespaces.iter())
    }
}
