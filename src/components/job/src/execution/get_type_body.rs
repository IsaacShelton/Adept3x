use super::Executable;
use crate::{
    Continuation, ExecutionCtx, Executor, ResolveType, Suspend, SuspendMany,
    repr::{
        DeclScope, EnumBody, EnumVariant, Field, Param, Params, StructBody, TraitBody, TraitFunc,
        Type, TypeAliasBody, TypeBody,
    },
};
use ast_workspace::{AstWorkspace, EnumRef, StructRef, TraitRef, TypeAliasRef, TypeDeclRef};
use by_address::ByAddress;
use derivative::Derivative;
use indexmap::IndexMap;
use source_files::Source;

#[derive(Clone, Derivative)]
#[derivative(Debug, PartialEq, Eq, Hash)]
pub struct GetTypeBody<'env> {
    type_decl_ref: TypeDeclRef,

    #[derivative(Debug = "ignore")]
    decl_scope: ByAddress<&'env DeclScope>,

    #[derivative(Debug = "ignore")]
    workspace: ByAddress<&'env AstWorkspace<'env>>,

    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    inner_types: SuspendMany<'env, &'env Type<'env>>,

    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    backing_type: Suspend<'env, &'env Type<'env>>,
}

impl<'env> GetTypeBody<'env> {
    pub fn new(
        workspace: &'env AstWorkspace<'env>,
        decl_scope: &'env DeclScope,
        type_decl_ref: TypeDeclRef,
    ) -> Self {
        Self {
            workspace: ByAddress(workspace),
            decl_scope: ByAddress(decl_scope),
            type_decl_ref,
            inner_types: None,
            backing_type: None,
        }
    }

    fn do_struct(
        self,
        executor: &Executor<'env>,
        ctx: &mut ExecutionCtx<'env>,
        idx: StructRef,
    ) -> Result<<Self as Executable<'env>>::Output, Continuation<'env>> {
        let def = &self.workspace.symbols.all_structs[idx];

        if let Some(fields_types) = executor.demand_many(&self.inner_types) {
            let fields = IndexMap::from_iter(def.fields.iter().zip(fields_types.into_iter()).map(
                |((name, ast_field), resolved_type)| {
                    (
                        name.as_str(),
                        Field {
                            ty: resolved_type,
                            privacy: ast_field.privacy,
                            source: ast_field.source,
                        },
                    )
                },
            ));

            return Ok(ctx.alloc(TypeBody::Struct(StructBody {
                fields,
                is_packed: def.is_packed,
                params: def.params.clone(),
                source: def.source,
            })));
        }

        suspend_many!(
            self.inner_types,
            executor.request_many(def.fields.iter().map(|(_name, field)| ResolveType::new(
                &self.workspace,
                &field.ast_type,
                &self.decl_scope
            )),),
            ctx
        )
    }

    fn do_enum(
        self,
        executor: &Executor<'env>,
        ctx: &mut ExecutionCtx<'env>,
        idx: EnumRef,
    ) -> Result<<Self as Executable<'env>>::Output, Continuation<'env>> {
        let def = &self.workspace.symbols.all_enums[idx];

        let Some(backing_type) = executor.demand(self.backing_type) else {
            return suspend!(
                self.backing_type,
                executor.request(ResolveType::new(
                    &self.workspace,
                    def.backing_type
                        .as_ref()
                        .unwrap_or_else(|| ctx.alloc(ast::TypeKind::u32().at(Source::internal()))),
                    &self.decl_scope
                )),
                ctx
            );
        };

        let variants = IndexMap::from_iter(def.members.iter().map(|(name, variant)| {
            (
                name.as_str(),
                EnumVariant {
                    value: variant.value.clone(),
                    explicit_value: variant.explicit_value,
                },
            )
        }));

        Ok(ctx.alloc(TypeBody::Enum(EnumBody {
            variants,
            backing_type,
            privacy: def.privacy,
            source: def.source,
        })))
    }

    fn do_alias(
        self,
        executor: &Executor<'env>,
        ctx: &mut ExecutionCtx<'env>,
        idx: TypeAliasRef,
    ) -> Result<<Self as Executable<'env>>::Output, Continuation<'env>> {
        let def = &self.workspace.symbols.all_type_aliases[idx];

        let Some(target) = executor.demand(self.backing_type) else {
            return suspend!(
                self.backing_type,
                executor.request(ResolveType::new(
                    &self.workspace,
                    &def.value,
                    &self.decl_scope
                )),
                ctx
            );
        };

        Ok(ctx.alloc(TypeBody::TypeAlias(TypeAliasBody {
            target,
            params: def.params.clone(),
            privacy: def.privacy,
            source: def.source,
        })))
    }

    fn do_trait(
        self,
        executor: &Executor<'env>,
        ctx: &mut ExecutionCtx<'env>,
        idx: TraitRef,
    ) -> Result<<Self as Executable<'env>>::Output, Continuation<'env>> {
        let def = &self.workspace.symbols.all_traits[idx];

        let Some(inner_types) = executor.demand_many(&self.inner_types) else {
            let all_inner_types = def
                .funcs
                .iter()
                .flat_map(|func| {
                    func.params
                        .required
                        .iter()
                        .map(|param| &param.ast_type)
                        .chain(std::iter::once(&func.return_type))
                })
                .map(|ast_type| ResolveType::new(&self.workspace, ast_type, &self.decl_scope));

            return suspend_many!(
                self.inner_types,
                executor.request_many(all_inner_types),
                ctx
            );
        };

        let mut inner_types = inner_types.iter();

        let funcs = IndexMap::from_iter(def.funcs.iter().map(|func| {
            (
                func.name.as_str(),
                TraitFunc {
                    params: Params {
                        required: ctx.alloc_slice_fill_iter(func.params.required.iter().map(
                            |param| Param {
                                name: param.name.as_ref().map(|name| name.as_str()),
                                ty: inner_types.next().unwrap(),
                            },
                        )),
                        is_cstyle_vararg: func.params.is_cstyle_vararg,
                    },
                    return_type: inner_types.next().unwrap(),
                    source: func.source,
                },
            )
        }));

        Ok(ctx.alloc(TypeBody::Trait(TraitBody {
            params: def.params.clone(),
            funcs,
            source: def.source,
            privacy: def.privacy,
        })))
    }
}

impl<'env> Executable<'env> for GetTypeBody<'env> {
    type Output = &'env TypeBody<'env>;

    fn execute(
        self,
        executor: &Executor<'env>,
        ctx: &mut ExecutionCtx<'env>,
    ) -> Result<Self::Output, Continuation<'env>> {
        match self.type_decl_ref {
            TypeDeclRef::Struct(idx) => self.do_struct(executor, ctx, idx),
            TypeDeclRef::Enum(idx) => self.do_enum(executor, ctx, idx),
            TypeDeclRef::Alias(idx) => self.do_alias(executor, ctx, idx),
            TypeDeclRef::Trait(idx) => self.do_trait(executor, ctx, idx),
        }
    }
}
