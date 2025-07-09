use super::{EstimateDeclScope, Executable, GetFuncBody};
use crate::{
    BuiltinTypes, Continuation, ExecutionCtx, Executor, Suspend, SuspendMany, SuspendManyAssoc,
    repr::{
        DeclScope, DeclScopeOrigin, FindTypeResult, FuncBody, FuncHead, TypeBody, TypeHead,
        TypeKind,
    },
};
use arena::Arena;
use ast_workspace::AstWorkspace;
use by_address::ByAddress;
use compiler::BuildOptions;
use derivative::Derivative;
use derive_more::Debug;
use itertools::Itertools;
use primitives::{FloatSize, IntegerBits, IntegerSign};
use source_files::Source;

#[derive(Debug, Derivative)]
#[derivative(PartialEq, Eq)]
#[debug("...")]
pub struct BuildWorkspace<'env> {
    pub workspace: ByAddress<&'env AstWorkspace<'env>>,

    #[derivative(Hash = "ignore")]
    #[derivative(PartialEq = "ignore")]
    pub module_scopes: SuspendManyAssoc<'env, DeclScopeOrigin, &'env DeclScope>,

    #[derivative(Hash = "ignore")]
    #[derivative(PartialEq = "ignore")]
    pub estimate_type_heads: SuspendManyAssoc<'env, DeclScopeOrigin, &'env [&'env TypeHead<'env>]>,

    #[derivative(Hash = "ignore")]
    #[derivative(PartialEq = "ignore")]
    pub find_type: Suspend<'env, FindTypeResult>,

    #[derivative(Hash = "ignore")]
    #[derivative(PartialEq = "ignore")]
    pub decl_scope: Option<&'env DeclScope>,

    #[derivative(Hash = "ignore")]
    #[derivative(PartialEq = "ignore")]
    pub type_body: Suspend<'env, &'env TypeBody<'env>>,

    #[derivative(Hash = "ignore")]
    #[derivative(PartialEq = "ignore")]
    pub func_head: Suspend<'env, &'env FuncHead<'env>>,

    #[derivative(Hash = "ignore")]
    #[derivative(PartialEq = "ignore")]
    pub func_body: Suspend<'env, &'env FuncBody<'env>>,

    #[derivative(Hash = "ignore")]
    #[derivative(PartialEq = "ignore")]
    pub builtin_types: Option<&'env BuiltinTypes<'env>>,

    #[derivative(Hash = "ignore")]
    #[derivative(PartialEq = "ignore")]
    pub build_options: &'env BuildOptions,

    #[derivative(Hash = "ignore")]
    #[derivative(PartialEq = "ignore")]
    pub all_funcs: SuspendMany<'env, &'env FuncBody<'env>>,
}

impl<'env> BuildWorkspace<'env> {
    pub fn new(workspace: &'env AstWorkspace<'env>, build_options: &'env BuildOptions) -> Self {
        Self {
            workspace: ByAddress(workspace),
            module_scopes: None,
            estimate_type_heads: None,
            find_type: None,
            decl_scope: None,
            type_body: None,
            func_head: None,
            func_body: None,
            builtin_types: None,
            build_options,
            all_funcs: None,
        }
    }
}

impl<'env> Executable<'env> for BuildWorkspace<'env> {
    type Output = ir::Module;

    #[allow(unused_variables)]
    fn execute(
        mut self,
        executor: &Executor<'env>,
        ctx: &mut ExecutionCtx<'env>,
    ) -> Result<Self::Output, Continuation<'env>> {
        let workspace = self.workspace.0;

        if self.builtin_types.is_none() {
            self.builtin_types = Some(
                ctx.alloc(BuiltinTypes {
                    bool: TypeKind::Boolean.at(Source::internal()),
                    i32: TypeKind::BitInteger(IntegerBits::Bits32, IntegerSign::Signed)
                        .at(Source::internal()),
                    u32: TypeKind::BitInteger(IntegerBits::Bits32, IntegerSign::Unsigned)
                        .at(Source::internal()),
                    i64: TypeKind::BitInteger(IntegerBits::Bits64, IntegerSign::Signed)
                        .at(Source::internal()),
                    u64: TypeKind::BitInteger(IntegerBits::Bits64, IntegerSign::Unsigned)
                        .at(Source::internal()),
                    f64: TypeKind::Floating(FloatSize::Bits64).at(Source::internal()),
                    never: TypeKind::Never.at(Source::internal()),
                }),
            );
        }

        let builtin_types = self.builtin_types.as_ref().unwrap();

        let Some(scopes) = executor.demand_many_assoc(&self.module_scopes) else {
            return suspend_many_assoc!(
                self.module_scopes,
                workspace
                    .modules
                    .keys()
                    .map(|module_ref| {
                        let scope_origin = DeclScopeOrigin::Module(module_ref);

                        (
                            scope_origin,
                            executor.request(EstimateDeclScope {
                                workspace: self.workspace,
                                scope_origin,
                            }),
                        )
                    })
                    .collect(),
                ctx
            );
        };
        dbg!(&scopes);
        dbg!(
            self.workspace
                .modules
                .iter()
                .flat_map(|(_, module)| module.files.iter())
                .flat_map(
                    |ast_file| self.workspace.symbols.all_conditional_name_scopes.get_span(
                        self.workspace.symbols.all_name_scopes[ast_file.names]
                            .conditonal_name_scopes
                    )
                )
                .collect_vec()
        );

        let Some(all_funcs) = executor.demand_many(&self.all_funcs) else {
            let func_refs = scopes.iter().flat_map(|(decl_scope_origin, decl_scope)| {
                decl_scope.iter_self().flat_map(|(_, decl_set)| {
                    decl_set
                        .func_decls()
                        .map(|func_ref| (func_ref, *decl_scope))
                })
            });

            let tasks = func_refs.map(|(func_ref, decl_scope)| {
                GetFuncBody::new(&self.workspace, func_ref, decl_scope, builtin_types)
            });

            return suspend_many!(self.all_funcs, executor.request_many(tasks), ctx);
        };

        dbg!(all_funcs);

        let funcs = Arena::new();

        // We won't worry about finding the interepreter entry point for now
        let interpreter_entry_point = None;

        let ir_module = ir::Module {
            interpreter_entry_point,
            target: self.build_options.target,
            funcs,
            structs: Arena::new(),
            globals: Arena::new(),
        };

        Ok(ir_module)
    }
}
