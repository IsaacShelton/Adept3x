use ast_workspace::{AstWorkspace, ModuleRef, NameScopeRef};
use smallvec::{SmallVec, smallvec};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum DeclScopeOrigin {
    Module(ModuleRef),
    NameScope(NameScopeRef),
}

impl DeclScopeOrigin {
    pub fn name_scopes(&self, workspace: &AstWorkspace) -> SmallVec<[NameScopeRef; 4]> {
        match self {
            DeclScopeOrigin::Module(module_ref) => {
                let module = &workspace.modules[*module_ref];
                module.files.iter().map(|file| file.names).collect()
            }
            DeclScopeOrigin::NameScope(name_scope_ref) => smallvec![*name_scope_ref],
        }
    }
}
