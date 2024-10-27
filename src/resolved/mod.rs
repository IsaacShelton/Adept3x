mod block;
mod datatype;
mod destination;
mod enumeration;
mod expr;
mod function;
mod global;
mod helper_expr;
mod human_name;
mod overload;
mod stmt;
mod structure;
mod type_decl;
mod variable_storage;

pub use self::variable_storage::VariableStorageKey;
pub use crate::ast::{IntegerBits, IntegerSign};
use crate::{ast::AstWorkspace, source_files::SourceFiles, workspace::fs::FsNodeId};
pub use block::*;
pub use datatype::*;
pub use destination::*;
pub use enumeration::*;
pub use expr::*;
pub use function::*;
pub use global::*;
pub use helper_expr::*;
pub use human_name::*;
pub use overload::*;
use slotmap::{new_key_type, SlotMap};
use std::collections::HashMap;
pub use stmt::*;
pub use structure::*;
pub use type_decl::*;
pub use variable_storage::*;

new_key_type! {
    pub struct FunctionRef;
    pub struct GlobalVarRef;
    pub struct StructureRef;
    pub struct EnumRef;
    pub struct TypeAliasRef;
}

#[derive(Clone, Debug)]
pub struct Ast<'a> {
    pub source_files: &'a SourceFiles,
    pub entry_point: Option<FunctionRef>,
    pub functions: SlotMap<FunctionRef, Function>,
    pub structures: SlotMap<StructureRef, Structure>,
    pub globals: SlotMap<GlobalVarRef, GlobalVar>,
    pub enums: SlotMap<EnumRef, Enum>,
    pub type_aliases: SlotMap<TypeAliasRef, Type>,
    pub workspace: &'a AstWorkspace<'a>,
    pub types_per_module: HashMap<FsNodeId, HashMap<String, TypeDecl>>,
}

impl<'a> Ast<'a> {
    const MAX_UNALIAS_DEPTH: usize = 1024;

    pub fn new(source_files: &'a SourceFiles, workspace: &'a AstWorkspace) -> Self {
        Self {
            source_files,
            entry_point: None,
            functions: SlotMap::with_key(),
            structures: SlotMap::with_key(),
            globals: SlotMap::with_key(),
            enums: SlotMap::with_key(),
            type_aliases: SlotMap::with_key(),
            workspace,
            types_per_module: HashMap::new(),
        }
    }

    pub fn unalias(&'a self, mut resolved_type: &'a Type) -> Result<&'a Type, UnaliasError> {
        let mut depth = 0;

        while let TypeKind::TypeAlias(_, type_alias_ref) = resolved_type.kind {
            resolved_type = self
                .type_aliases
                .get(type_alias_ref)
                .expect("valid type alias ref");

            depth += 1;

            if depth > Self::MAX_UNALIAS_DEPTH {
                return Err(UnaliasError::MaxDepthExceeded);
            }
        }

        Ok(resolved_type)
    }
}

#[derive(Clone, Debug)]
pub enum UnaliasError {
    MaxDepthExceeded,
}
