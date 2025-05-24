use super::Executable;
use crate::{
    Continuation, ExecutionCtx, Executor, ResolveType, Suspend, SuspendMany,
    repr::{DeclScope, EnumBody, EnumVariant, Field, StructBody, Type, TypeBody},
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
    field_types: SuspendMany<'env, &'env Type<'env>>,

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
            field_types: None,
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

        if let Some(fields_types) = executor.demand_many(&self.field_types) {
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
            self.field_types,
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
        _executor: &Executor<'env>,
        _ctx: &mut ExecutionCtx<'env>,
        idx: TypeAliasRef,
    ) -> Result<<Self as Executable<'env>>::Output, Continuation<'env>> {
        todo!("do_alias {:?}", idx)
    }

    fn do_trait(
        self,
        _executor: &Executor<'env>,
        _ctx: &mut ExecutionCtx<'env>,
        idx: TraitRef,
    ) -> Result<<Self as Executable<'env>>::Output, Continuation<'env>> {
        todo!("do_trait {:?}", idx)
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
