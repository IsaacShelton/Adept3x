use crate::{
    Continuation, Executable, ExecutionCtx, Executor, Suspend, TypeSearch,
    execution::resolve::ResolveNamespace,
    module_graph::ModuleView,
    repr::{DeclHead, DeclHeadTypeLike, Type, TypeKind, UnaliasedType, UserDefinedType},
    sub_task::SubTask,
};
use by_address::ByAddress;
use derivative::Derivative;
use diagnostics::ErrorDiagnostic;

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
            ast::TypeKind::Named(name_path, type_args) => {
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

                let DeclHead::TypeLike(DeclHeadTypeLike::Type(type_head)) = symbol else {
                    return Err(ErrorDiagnostic::new(
                        format!("Symbol `{}` must be a type", basename),
                        self.ast_type.source,
                    )
                    .into());
                };

                if type_args.len() != 0 {
                    return Err(ErrorDiagnostic::new("Type args on user-defined types are not supported for new type resolution system yet", self.ast_type.source).into());
                }

                let udt = TypeKind::UserDefined(UserDefinedType {
                    name: type_head.name,
                    rest: type_head.rest,
                    args: &[],
                });

                // TODO: We need to unalias any type aliases here...

                // TODO: We also need to handle type args here...

                udt
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
