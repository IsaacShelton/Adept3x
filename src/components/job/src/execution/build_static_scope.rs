use super::Execute;
use crate::{
    Artifact, Executor, Progress, TaskRef,
    prereqs::Prereqs,
    repr::{Decl, StaticScope, TypeRef},
};
use fs_tree::FsNodeId;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct BuildStaticScope<'env> {
    pub ast_workspace: TaskRef<'env>,
    pub fs_node_id: FsNodeId,
}

impl<'env> Prereqs<'env> for BuildStaticScope<'env> {
    fn prereqs(&self) -> Vec<TaskRef<'env>> {
        vec![self.ast_workspace]
    }
}

impl<'env> Execute<'env> for BuildStaticScope<'env> {
    fn execute(self, executor: &Executor<'env>) -> Progress<'env> {
        let mut scope = StaticScope {
            ..Default::default()
        };

        let workspace = executor.truth.read().unwrap().tasks[self.ast_workspace]
            .state
            .completed()
            .unwrap()
            .unwrap_ast_workspace();

        let ast_file = &workspace.files.get(self.fs_node_id).unwrap();

        for func_id in ast_file.funcs {
            let decl = Decl::Func(func_id);
            let name = &workspace.all_funcs[func_id].head.name;
            scope.names.entry(name.into()).or_default().push(decl);
        }

        for enum_id in ast_file.enums {
            let decl = Decl::Type(TypeRef::Enum(enum_id));
            let name = &workspace.all_enums[enum_id].name;
            scope.names.entry(name.into()).or_default().push(decl);
        }

        for impl_id in ast_file.impls {
            let decl = Decl::Impl(impl_id);
            if let Some(name) = workspace.all_impls[impl_id].name.as_ref() {
                scope.names.entry(name.into()).or_default().push(decl);
            }
        }

        for trait_id in ast_file.traits {
            let decl = Decl::Type(TypeRef::Trait(trait_id));
            let name = &workspace.all_traits[trait_id].name;
            scope.names.entry(name.into()).or_default().push(decl);
        }

        for struct_id in ast_file.structs {
            let decl = Decl::Type(TypeRef::Struct(struct_id));
            let name = &workspace.all_structs[struct_id].name;
            scope.names.entry(name.into()).or_default().push(decl);
        }

        for enum_id in ast_file.enums {
            let decl = Decl::Type(TypeRef::Enum(enum_id));
            let name = &workspace.all_enums[enum_id].name;
            scope.names.entry(name.into()).or_default().push(decl);
        }

        for type_alias_id in ast_file.type_aliases {
            let decl = Decl::Type(TypeRef::Alias(type_alias_id));
            let name = &workspace.all_type_aliases[type_alias_id].name;
            scope.names.entry(name.into()).or_default().push(decl);
        }

        for global_id in ast_file.globals {
            let decl = Decl::Global(global_id);
            let name = &workspace.all_globals[global_id].name;
            scope.names.entry(name.into()).or_default().push(decl);
        }

        for expr_alias_id in ast_file.expr_aliases {
            let decl = Decl::ExprAlias(expr_alias_id);
            let name = &workspace.all_expr_aliases[expr_alias_id].name;
            scope.names.entry(name.into()).or_default().push(decl);
        }

        Artifact::StaticScope(scope).into()
    }
}
