use crate::{
    Continuation, Executable, ExecutionCtx, Executor, Suspend,
    execution::resolve::ResolveType,
    module_graph::ModuleView,
    repr::{Field, StructBody, UnaliasedType},
};
use by_address::ByAddress;
use derivative::Derivative;
use indexmap::IndexMap;

#[derive(Clone, Derivative)]
#[derivative(Debug, PartialEq, Eq, Hash)]
pub struct ResolveStructureBody<'env> {
    view: &'env ModuleView<'env>,
    structure: ByAddress<&'env ast::Struct>,

    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    fields: IndexMap<&'env str, Field<'env>>,

    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    resolved_type: Suspend<'env, UnaliasedType<'env>>,
}

impl<'env> ResolveStructureBody<'env> {
    pub fn new(view: &'env ModuleView<'env>, structure: &'env ast::Struct) -> Self {
        Self {
            view,
            structure: ByAddress(structure),
            fields: IndexMap::with_capacity(structure.fields.len()),
            resolved_type: None,
        }
    }
}

impl<'env> Executable<'env> for ResolveStructureBody<'env> {
    type Output = &'env StructBody<'env>;

    fn execute(
        mut self,
        executor: &Executor<'env>,
        ctx: &mut ExecutionCtx<'env>,
    ) -> Result<Self::Output, Continuation<'env>> {
        while self.fields.len() < self.structure.fields.len() {
            let (field_name, field) = self.structure.fields.get_index(self.fields.len()).unwrap();

            let Some(resolved_type) = executor.demand(self.resolved_type) else {
                return suspend!(
                    self.resolved_type,
                    executor.request(ResolveType::new(self.view, &field.ast_type)),
                    ctx
                );
            };

            self.fields.insert(
                field_name.as_str(),
                Field {
                    ty: resolved_type,
                    privacy: field.privacy,
                    source: field.source,
                },
            );
            self.resolved_type = None;
        }

        Ok(ctx.alloc(StructBody {
            fields: std::mem::take(&mut self.fields),
            is_packed: self.structure.is_packed,
            params: &self.structure.params,
            source: self.structure.source,
        }))
    }
}
