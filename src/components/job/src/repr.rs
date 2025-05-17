use fs_tree::FsNodeId;
use smallvec::SmallVec;
use std::collections::{HashMap, hash_map::Entry};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct DeclScopeRef {
    file: FsNodeId,
}

impl DeclScopeRef {
    pub fn new(file: FsNodeId) -> Self {
        Self { file }
    }
}

/// A collection of identifiers mapped to declaration sets
#[derive(Debug)]
pub struct DeclScope {
    parent: Option<DeclScopeRef>,
    names: HashMap<String, DeclSet>,
}

impl DeclScope {
    pub fn new() -> Self {
        Self {
            parent: None,
            names: Default::default(),
        }
    }

    pub fn get(&self, name: &str) -> Option<&DeclSet> {
        self.names.get(name).as_ref().copied()
    }

    pub fn entry(&mut self, name: String) -> Entry<String, DeclSet> {
        self.names.entry(name)
    }

    pub fn push_unique(&mut self, name: String, decl: Decl) {
        self.names.entry(name).or_default().push_unique(decl);
    }

    pub fn parent(&self) -> Option<DeclScopeRef> {
        self.parent
    }
}

/// A group of declarations under the same name
#[derive(Debug, Default)]
pub struct DeclSet(SmallVec<[Decl; 4]>);

impl<'env> DeclSet {
    pub fn push_unique(&mut self, decl: Decl) {
        self.0.push(decl);
    }
}

/// An abstract reference to an AST type declaration
#[derive(Debug)]
pub enum TypeRef {
    Struct(ast_workspace::StructRef),
    Enum(ast_workspace::EnumRef),
    Alias(ast_workspace::TypeAliasRef),
    Trait(ast_workspace::TraitRef),
}

/// A symbol declaration
#[derive(Debug)]
pub enum Decl {
    Global(ast_workspace::GlobalRef),
    Func(ast_workspace::FuncRef),
    Type(TypeRef),
    Impl(ast_workspace::ImplRef),
    Namespace(ast_workspace::NameScopeRef),
    ExprAlias(ast_workspace::ExprAliasRef),
}
