use super::Execute;
use crate::{Artifact, Executor, Progress, Progression};
use ast_workspace::AstWorkspace;
use fs_tree::FsNodeId;
use std::collections::HashMap;

#[derive(Debug)]
pub struct IdentifiersHashMap<'outside>(pub &'outside AstWorkspace<'outside>, pub FsNodeId);

impl<'outside> Execute<'outside> for IdentifiersHashMap<'outside> {
    fn execute(self, _executor: &Executor<'outside>) -> Progress<'outside> {
        let ast_workspace = self.0;
        let fs_node_id = self.1;
        let mut identifiers = HashMap::new();

        let duplicate_name = |name| {
            return Progression::Error(format!("Multiple definitions for '{}'", name)).into();
        };

        let file = ast_workspace
            .files
            .get(fs_node_id)
            .expect("file exists in workspace");

        for func in &ast_workspace.all_funcs[file.funcs] {
            if identifiers
                .insert(func.head.name.as_str().into(), ())
                .is_some()
            {
                return duplicate_name(&func.head.name);
            }
        }

        for structure in &ast_workspace.all_structs[file.structs] {
            if identifiers
                .insert(structure.name.as_str().into(), ())
                .is_some()
            {
                return duplicate_name(&structure.name);
            }
        }

        for enumeration in &ast_workspace.all_enums[file.enums] {
            if identifiers
                .insert(enumeration.name.as_str().into(), ())
                .is_some()
            {
                return duplicate_name(&enumeration.name);
            }
        }

        for global in &ast_workspace.all_globals[file.globals] {
            if identifiers
                .insert(global.name.as_str().into(), ())
                .is_some()
            {
                return duplicate_name(&global.name);
            }
        }

        for type_alias in &ast_workspace.all_type_aliases[file.type_aliases] {
            if identifiers
                .insert(type_alias.name.as_str().into(), ())
                .is_some()
            {
                return duplicate_name(&type_alias.name);
            }
        }

        for expr_alias in &ast_workspace.all_expr_aliases[file.expr_aliases] {
            if identifiers
                .insert(expr_alias.name.as_str().into(), ())
                .is_some()
            {
                return duplicate_name(&expr_alias.name);
            }
        }

        for trait_def in &ast_workspace.all_traits[file.traits] {
            if identifiers
                .insert(trait_def.name.as_str().into(), ())
                .is_some()
            {
                return duplicate_name(&trait_def.name);
            }
        }

        for implementation in &ast_workspace.all_impls[file.impls] {
            let Some(name) = &implementation.name else {
                continue;
            };

            if identifiers.insert(name.as_str().into(), ()).is_some() {
                return duplicate_name(name);
            }
        }

        Artifact::Identifiers(identifiers).into()
    }
}
