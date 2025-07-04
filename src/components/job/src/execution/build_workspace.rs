use super::Executable;
use crate::{
    BuiltinTypes, Continuation, ExecutionCtx, Executor, Suspend, SuspendManyAssoc,
    repr::{
        DeclScope, DeclScopeOrigin, FindTypeResult, FuncBody, FuncHead, TypeBody, TypeHead,
        TypeKind,
    },
};
use arena::Arena;
use ast_workspace::AstWorkspace;
use attributes::Tag;
use by_address::ByAddress;
use compiler::BuildOptions;
use derivative::Derivative;
use derive_more::Debug;
use diagnostics::ErrorDiagnostic;
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

        // We will want to spawn tasks to compute the
        // function bodies for every function.

        // We will also want to then spawn a task to compute the IR
        // for the program,
        // which itself depends on monomorphizing, globals, type layouts, etc.

        /*
        if let Some(func_body) = executor.demand(self.func_body) {
            dbg!(func_body);
            return Ok(ctx.alloc(asg::Asg::new(self.workspace.0)));
        }

        if let Some(func_head) = executor.demand(self.func_head) {
            // dbg!(func_head);

            let func_ref = self
                .workspace
                .symbols
                .all_funcs
                .iter()
                .filter(|(_, func)| func.head.name == "exampleFunction")
                .map(|(func_ref, _)| func_ref)
                .next()
                .unwrap();

            return suspend!(
                self.func_body,
                executor.request(GetFuncBody::new(
                    workspace,
                    func_ref,
                    self.decl_scope.unwrap(),
                    builtin_types,
                )),
                ctx
            );
        }

        if let Some(type_body) = executor.demand(self.type_body) {
            // dbg!(type_body);

            let func_ref = self
                .workspace
                .symbols
                .all_funcs
                .iter()
                .filter(|(_, func)| func.head.name == "exampleFunction")
                .map(|(func_ref, _)| func_ref)
                .next()
                .unwrap();

            return suspend!(
                self.func_head,
                executor.request(GetFuncHead::new(
                    workspace,
                    func_ref,
                    self.decl_scope.unwrap()
                )),
                ctx
            );
        }

        if let Some(found) = executor.demand(self.find_type) {
            // dbg!(&found);

            if let Ok(Some(type_decl_ref)) = found {
                return suspend!(
                    self.type_body,
                    executor.request(GetTypeBody::new(
                        workspace,
                        self.decl_scope.unwrap(),
                        type_decl_ref
                    )),
                    ctx
                );
            } else {
                return Ok(ctx.alloc(asg::Asg::new(self.workspace.0)));
            }
        }

        if let Some(_type_heads) = executor.demand_many_assoc(&self.estimate_type_heads) {
            return suspend!(
                self.find_type,
                executor.request(FindType::new(
                    workspace,
                    self.decl_scope.unwrap(),
                    "MyTrait",
                    0
                )),
                ctx
            );
        }

        if let Some(scopes) = executor.demand_many_assoc(&self.module_scopes) {
            let first_module_ref = self.workspace.modules.keys().next().unwrap();
            self.decl_scope = scopes
                .iter()
                .find(|scope| scope.0 == DeclScopeOrigin::Module(first_module_ref))
                .map(|(_k, v)| &**v);

            return suspend_many_assoc!(
                self.estimate_type_heads,
                scopes
                    .iter()
                    .map(|(origin, scope)| (
                        *origin,
                        executor.request(EstimateTypeHeads::new(workspace, scope, "Test"))
                    ))
                    .collect(),
                ctx
            );
        }

        suspend_many_assoc!(
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
        )
        */

        let mut main_modules = self
            .workspace
            .modules
            .iter()
            .filter(|(module_ref, module)| {
                module
                    .files
                    .iter()
                    .map(|file| &self.workspace.symbols.all_name_scopes[file.names])
                    .any(|name_scope| {
                        self.workspace
                            .symbols
                            .all_funcs
                            .get_span(name_scope.funcs)
                            .any(|f| f.head.tag == Some(Tag::Main))
                    })
            });

        let Some((main_module_ref, main_module)) = main_modules.next() else {
            return Err(ErrorDiagnostic::new("Missing 'main' function", Source::internal()).into());
        };

        if main_modules.next().is_some() {
            return Err(
                ErrorDiagnostic::new("Multiple main modules exist", Source::internal()).into(),
            );
        }

        // Resolved Type -> Resolved Type w/ Layout -> IR Layout => IR

        dbg!(&main_module);
        //todo!("lower main module");

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
