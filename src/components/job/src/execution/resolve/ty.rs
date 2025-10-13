use crate::{
    Continuation, Executable, ExecutionCtx, Executor, Suspend, TypeSearch,
    execution::resolve::ResolveNamespace,
    module_graph::ModuleView,
    repr::{Type, TypeKind, UnaliasedType},
    sub_task::SubTask,
};
use by_address::ByAddress;
use derivative::Derivative;

#[derive(Clone, Derivative)]
#[derivative(Debug, PartialEq, Eq, Hash)]
pub struct ResolveType<'env> {
    ast_type: ByAddress<&'env ast::Type>,

    #[derivative(Debug = "ignore")]
    view: &'env ModuleView<'env>,

    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    inner_type: Suspend<'env, UnaliasedType<'env>>,

    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    resolved_namespace: Option<ResolveNamespace<'env>>,
    /*
    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    inner_find_type: Suspend<'env, FindTypeResult>,

    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    type_args: SuspendMany<'env, &'env TypeArg<'env>>,
    */
}

impl<'env> ResolveType<'env> {
    pub fn new(view: &'env ModuleView<'env>, ast_type: &'env ast::Type) -> Self {
        Self {
            ast_type: ByAddress(ast_type),
            view,
            inner_type: None,
            resolved_namespace: None,
        }
    }
}

impl<'env> Executable<'env> for ResolveType<'env> {
    type Output = UnaliasedType<'env>;

    fn execute(
        mut self,
        executor: &Executor<'env>,
        ctx: &mut ExecutionCtx<'env>,
    ) -> Result<Self::Output, Continuation<'env>> {
        let kind = match &self.ast_type.kind {
            ast::TypeKind::Boolean => TypeKind::Boolean,
            ast::TypeKind::Integer(bits, sign) => TypeKind::BitInteger(*bits, *sign),
            ast::TypeKind::CInteger(cinteger, sign) => TypeKind::CInteger(*cinteger, *sign),
            ast::TypeKind::SizeInteger(sign) => TypeKind::SizeInteger(*sign),
            ast::TypeKind::Floating(size) => TypeKind::Floating(*size),
            ast::TypeKind::Ptr(inner) => {
                let Some(inner) = executor.demand(self.inner_type) else {
                    return suspend!(
                        self.inner_type,
                        executor.request(ResolveType::new(self.view, inner)),
                        ctx
                    );
                };

                TypeKind::Ptr(inner.0)
            }
            ast::TypeKind::Deref(inner) => {
                let Some(inner) = executor.demand(self.inner_type) else {
                    return suspend!(
                        self.inner_type,
                        executor.request(ResolveType::new(self.view, inner)),
                        ctx
                    );
                };

                TypeKind::Deref(inner.0)
            }
            ast::TypeKind::FixedArray(_) => {
                unimplemented!("we don't resolve fixed array types yet")
            }
            ast::TypeKind::Void => TypeKind::Void,
            ast::TypeKind::Never => TypeKind::Never,
            ast::TypeKind::Named(name_path, _type_args) => {
                let basename = name_path.basename();

                let view = if name_path.has_namespace() {
                    let resolved_namespace = self.resolved_namespace.get_or_insert_with(|| {
                        ResolveNamespace::new(self.view, self.ast_type.source)
                    });

                    execute_sub_task!(
                        self,
                        resolved_namespace,
                        executor,
                        ctx,
                        name_path.namespaces()
                    )
                    .expect("infallible")
                } else {
                    *self.view
                };

                let symbol = match view.find_symbol(
                    executor,
                    TypeSearch {
                        name: basename,
                        source: self.ast_type.source,
                    },
                ) {
                    Ok(symbol) => symbol,
                    Err(into_continuation) => return Err(into_continuation(self.into())),
                };

                todo!("resolve named type - {:?} {:?}", basename, symbol);

                // NOTE: We will also need to unalias here,
                // although perhaps we have a separate option
                // for this to enable the preservation of type alises.

                /*
                let Some(name) = name.as_plain_str() else {
                    unimplemented!("we don't handle namespaced types yet");
                };

                let Some(inner) = executor.demand(self.inner_find_type) else {
                    return suspend!(
                        self.inner_find_type,
                        executor.request(FindType::new(
                            workspace,
                            self.view,
                            name,
                            type_args.len()
                        )),
                        ctx
                    );
                };

                let Ok(Some(found)) = inner else {
                    unimplemented!("we don't report errors yet for failing to find a type!");
                };

                let Some(type_args) = executor.demand_many(&self.type_args) else {
                    return suspend_many!(
                        self.type_args,
                        executor.request_many(type_args.iter().map(|type_arg| {
                            ResolveTypeArg::new(workspace, type_arg, self.view)
                        })),
                        ctx
                    );
                };

                TypeKind::UserDefined(UserDefinedType {
                    name: name.into(),
                    type_decl_ref: found,
                    args: ctx.alloc_slice_fill_iter(type_args.into_iter().cloned()),
                })
                */
            }
            ast::TypeKind::AnonymousStruct(_) => {
                unimplemented!("we don't resolve anonymous structs yet")
            }
            ast::TypeKind::AnonymousUnion(_) => {
                unimplemented!("we don't resolve anonymous unions yet")
            }
            ast::TypeKind::AnonymousEnum(_) => {
                unimplemented!("we don't resolve anonymous enums yet")
            }
            ast::TypeKind::FuncPtr(_) => {
                unimplemented!("we don't resolve function pointer types yet")
            }
            ast::TypeKind::Polymorph(name) => TypeKind::Polymorph(name),
        };

        Ok(UnaliasedType(ctx.alloc(Type {
            kind,
            source: self.ast_type.source,
        })))
    }
}
