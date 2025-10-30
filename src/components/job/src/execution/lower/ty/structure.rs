use crate::{
    Continuation, Executable, ExecutionCtx, Executor, SuspendMany, execution::lower::LowerType, ir,
    module_graph::ModuleView, repr::StructBody,
};
use by_address::ByAddress;
use derivative::Derivative;

#[derive(Clone, Derivative)]
#[derivative(Debug, PartialEq, Eq, Hash)]
pub struct LowerTypeStructure<'env> {
    view: &'env ModuleView<'env>,
    structure: ByAddress<&'env ast::Struct>,
    struct_body: ByAddress<&'env StructBody<'env>>,

    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    lowered_types: SuspendMany<'env, ir::Type<'env>>,
}

impl<'env> LowerTypeStructure<'env> {
    pub fn new(
        view: &'env ModuleView<'env>,
        structure: &'env ast::Struct,
        struct_body: &'env StructBody<'env>,
    ) -> Self {
        Self {
            view,
            structure: ByAddress(structure),
            struct_body: ByAddress(struct_body),
            lowered_types: None,
        }
    }
}

impl<'env> Executable<'env> for LowerTypeStructure<'env> {
    type Output = ir::Type<'env>;

    fn execute(
        self,
        executor: &Executor<'env>,
        ctx: &mut ExecutionCtx<'env>,
    ) -> Result<Self::Output, Continuation<'env>> {
        let ir = self.view.graph(|graph| graph.ir);

        let Some(lowered_types) = executor.demand_many(&self.lowered_types) else {
            return suspend_many!(
                self.lowered_types,
                self.struct_body
                    .fields
                    .values()
                    .map(|field| { executor.request(LowerType::new(self.view, &field.ty.0)) })
                    .collect(),
                ctx
            );
        };

        let fields = self
            .struct_body
            .fields
            .values()
            .zip(lowered_types.into_iter())
            .map(|(field, ir_type)| ir::Field {
                ir_type,
                properties: ir::FieldProperties::default(),
                source: field.source,
            });

        let struct_ref = ir.structs.alloc(ir::Struct {
            friendly_record_name: &self.structure.name,
            fields: ctx.alloc_slice_fill_iter(fields),
            is_packed: self.struct_body.is_packed,
            source: self.struct_body.source,
        });

        Ok(ir::Type::Struct(struct_ref))
    }
}
