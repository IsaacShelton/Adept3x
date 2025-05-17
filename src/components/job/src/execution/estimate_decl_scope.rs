use super::Execute;
use crate::{
    Artifact, Executor, Progress, TaskRef,
    prereqs::Prereqs,
    repr::{Decl, DeclScope, TypeRef},
};
use ast_workspace::{AstWorkspace, ModuleRef, NameScopeRef};
use by_address::ByAddress;
use smallvec::{SmallVec, smallvec};

/*
// NOTE: Maybe "Find" tasks are functions instead of tasks? Like sub-tasks that
// the parent task is responsible for advancing? (No dedicated artifact result)

pub enum FindTypeSubTask {}

impl FindTypeSubTask {
    fn resume() -> Option<FuncId> {}
}

let Some(func_id) = self.find_func.resume() else {
    return self;
};
*/

/*
    Tasks:
    FindType
    FindFunc
    FindImpl
    EstimateDeclScope                    - Gets initially known names in decl scope and its parent
    SearchExpandedDeclScope(name)        - Gets a generated decl set by name from a decl scope, or none if never generated
    Incorporate("scope1", "alias_star_1"),
    FuncHead,
    FuncBody,
    TypeHead,
    TypeBody,

    {
        alias * = createRealFunctions()
        alias * = createHelperFunctions()

        alias printHelloWorld = generate()

        func test() {
            printHelloWorld()
        }
    }

    FuncBody("test")
    FindFunc("printHelloWorld")
        EstimateDeclScope("scope1")
        SearchExpandedDeclScope("scope1", "printHelloWorld")
            // ...
            FindFunc("generate")
                EstimateDeclScope("scope1")
                SearchExpandedDeclScope("scope1", "generate")
            // ...

    FuncBody("test")
    FindFunc("printHelloWorld")
        EstimateDeclScope("scope1")
        IsDeclFunc("scope1", "printHelloWorld")
            FindFunc("generate")
                EstimateDeclScope("scope1")
                SearchExpandedDeclScope("scope1", "generate")
                    ...
                    Incorporate() Incorporate() Incorporate()

*/

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

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct EstimateDeclScope<'env> {
    pub workspace: ByAddress<&'env AstWorkspace<'env>>,
    pub scope_origin: DeclScopeOrigin,
}

impl<'env> Prereqs<'env> for EstimateDeclScope<'env> {
    fn prereqs(&self) -> Vec<TaskRef<'env>> {
        vec![]
    }
}

impl<'env> Execute<'env> for EstimateDeclScope<'env> {
    fn execute(self, _executor: &Executor<'env>) -> Progress<'env> {
        let workspace = self.workspace;
        let mut scope = DeclScope::new();

        for name_scope in self
            .scope_origin
            .name_scopes(&workspace)
            .into_iter()
            .map(|name_scope_ref| &workspace.symbols.all_name_scopes[name_scope_ref])
        {
            for func_id in name_scope.funcs.iter() {
                let name = &workspace.symbols.all_funcs[func_id].head.name;
                scope.push_unique(name.into(), Decl::Func(func_id));
            }

            for impl_id in name_scope.impls.iter() {
                if let Some(name) = workspace.symbols.all_impls[impl_id].name.as_ref() {
                    scope.push_unique(name.into(), Decl::Impl(impl_id));
                }
            }

            for trait_id in name_scope.traits.iter() {
                let name = &workspace.symbols.all_traits[trait_id].name;
                scope.push_unique(name.into(), Decl::Type(TypeRef::Trait(trait_id)));
            }

            for struct_id in name_scope.structs.iter() {
                let name = &workspace.symbols.all_structs[struct_id].name;
                scope.push_unique(name.into(), Decl::Type(TypeRef::Struct(struct_id)));
            }

            for enum_id in name_scope.enums.iter() {
                let name = &workspace.symbols.all_enums[enum_id].name;
                scope.push_unique(name.into(), Decl::Type(TypeRef::Enum(enum_id)));
            }

            for type_alias_id in name_scope.type_aliases.iter() {
                let name = &workspace.symbols.all_type_aliases[type_alias_id].name;
                scope.push_unique(name.into(), Decl::Type(TypeRef::Alias(type_alias_id)));
            }

            for global_id in name_scope.globals.iter() {
                let name = &workspace.symbols.all_globals[global_id].name;
                scope.push_unique(name.into(), Decl::Global(global_id));
            }

            for expr_alias_id in name_scope.expr_aliases.iter() {
                let name = &workspace.symbols.all_expr_aliases[expr_alias_id].name;
                scope.push_unique(name.into(), Decl::ExprAlias(expr_alias_id));
            }

            for namespace_id in name_scope.namespaces.iter() {
                let namespace = &workspace.symbols.all_namespaces[namespace_id];
                scope.push_unique(namespace.name.clone(), Decl::Namespace(namespace.names));
            }
        }

        Artifact::EstimatedDeclScope(scope).into()
    }
}
