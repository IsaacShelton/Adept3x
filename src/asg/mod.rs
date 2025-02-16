mod block;
mod datatype;
mod destination;
mod enumeration;
mod expr;
mod func;
mod generic_trait_ref;
mod global;
mod helper_expr;
mod human_name;
mod impl_params;
mod implementation;
mod overload;
mod stmt;
mod structure;
mod trait_constraint;
mod type_alias;
mod type_decl;
mod variable_storage;

pub use self::variable_storage::VariableStorageKey;
pub use crate::ast::{IntegerBits, IntegerSign};
use crate::{
    ast::AstWorkspace,
    resolve::{PolyRecipe, PolyValue, PolymorphErrorKind},
    source_files::SourceFiles,
};
pub use block::*;
pub use datatype::*;
pub use destination::*;
pub use enumeration::*;
pub use expr::*;
pub use func::*;
pub use generic_trait_ref::*;
pub use global::*;
pub use helper_expr::*;
pub use human_name::*;
pub use impl_params::*;
pub use implementation::*;
use indexmap::IndexMap;
pub use overload::*;
use slotmap::{new_key_type, SlotMap};
use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
};
pub use stmt::*;
pub use structure::*;
pub use trait_constraint::*;
pub use type_alias::TypeAlias;
pub use type_decl::*;
pub use variable_storage::*;

new_key_type! {
    pub struct FuncRef;
    pub struct GlobalVarRef;
    pub struct StructRef;
    pub struct EnumRef;
    pub struct TypeAliasRef;
    pub struct TraitRef;
    pub struct ImplRef;
}

#[derive(Clone, Debug)]
pub struct Asg<'a> {
    pub source_files: &'a SourceFiles,
    pub entry_point: Option<FuncRef>,
    pub funcs: SlotMap<FuncRef, Func>,
    pub structs: SlotMap<StructRef, Struct>,
    pub globals: SlotMap<GlobalVarRef, GlobalVar>,
    pub enums: SlotMap<EnumRef, Enum>,
    pub type_aliases: SlotMap<TypeAliasRef, TypeAlias>,
    pub traits: SlotMap<TraitRef, Trait>,
    pub impls: SlotMap<ImplRef, Impl>,
    pub workspace: &'a AstWorkspace<'a>,
}

impl<'a> Asg<'a> {
    const MAX_UNALIAS_DEPTH: usize = 1024;

    pub fn new(source_files: &'a SourceFiles, workspace: &'a AstWorkspace) -> Self {
        Self {
            source_files,
            entry_point: None,
            funcs: SlotMap::with_key(),
            structs: SlotMap::with_key(),
            globals: SlotMap::with_key(),
            enums: SlotMap::with_key(),
            type_aliases: SlotMap::with_key(),
            traits: SlotMap::with_key(),
            impls: SlotMap::with_key(),
            workspace,
        }
    }

    pub fn unalias(&'a self, whole_type: &'a Type) -> Result<Cow<'a, Type>, UnaliasError> {
        let mut running = Cow::Borrowed(whole_type);
        let mut depth = 0;

        while let TypeKind::TypeAlias(human_name, type_alias_ref, type_args) = &running.kind {
            let alias = self
                .type_aliases
                .get(*type_alias_ref)
                .expect("valid type alias ref");

            if type_args.len() != alias.params.len() {
                return Err(UnaliasError::IncorrectNumberOfTypeArgsForAlias(
                    human_name.0.clone(),
                ));
            }

            if alias.params.is_empty() {
                running = Cow::Borrowed(&alias.becomes);
            } else {
                let polymorphs = IndexMap::<String, PolyValue>::from_iter(
                    alias
                        .params
                        .iter()
                        .cloned()
                        .zip(type_args.iter().cloned().map(PolyValue::Type)),
                );

                let recipe = PolyRecipe { polymorphs };

                running = Cow::Owned(
                    recipe
                        .resolve_type(&alias.becomes)
                        .map_err(|e| UnaliasError::PolymorphError(e.kind))?,
                )
            }

            depth += 1;

            if depth > Self::MAX_UNALIAS_DEPTH {
                return Err(self.find_type_alias_self_reference(whole_type));
            }
        }

        Ok(running)
    }

    fn find_type_alias_self_reference(&self, whole_type: &Type) -> UnaliasError {
        let mut seen = HashSet::new();
        let mut running = whole_type;
        let mut depth = 0;

        while let TypeKind::TypeAlias(human_name, type_alias_ref, type_args) = &running.kind {
            let alias = self
                .type_aliases
                .get(*type_alias_ref)
                .expect("valid type alias ref");

            if !type_args.is_empty() || !alias.params.is_empty() {
                todo!("unalias type alias with type args");
            }

            running = &alias.becomes;

            if !seen.insert(type_alias_ref) {
                return UnaliasError::SelfReferentialTypeAlias(human_name.0.clone());
            }

            depth += 1;

            if depth > Self::MAX_UNALIAS_DEPTH {
                break;
            }
        }

        return UnaliasError::MaxDepthExceeded;
    }
}

#[derive(Clone, Debug)]
pub enum UnaliasError {
    MaxDepthExceeded,
    SelfReferentialTypeAlias(String),
    IncorrectNumberOfTypeArgsForAlias(String),
    PolymorphError(PolymorphErrorKind),
}
