mod structure;

use crate::{
    Continuation, Executable, ExecutionCtx, Executor, Suspend,
    execution::{
        lower::bits_and_sign_for_invisible_integer_in_range, resolve::ResolveStructureBody,
    },
    ir,
    module_graph::ModuleView,
    repr::{StructBody, Type, TypeHeadRestKind, TypeKind},
    target_layout::TargetLayout,
};
use by_address::ByAddress;
use derivative::Derivative;
use diagnostics::ErrorDiagnostic;
use primitives::IntegerBits;
pub use structure::LowerTypeStructure;

#[derive(Clone, Derivative)]
#[derivative(Debug, PartialEq, Eq, Hash)]
pub struct LowerType<'env> {
    view: &'env ModuleView<'env>,

    ty: ByAddress<&'env Type<'env>>,

    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    inner_type: Suspend<'env, ir::Type<'env>>,

    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    resolved_struct_body: Suspend<'env, &'env StructBody<'env>>,
}

impl<'env> LowerType<'env> {
    pub fn new(view: &'env ModuleView<'env>, ty: &'env Type<'env>) -> Self {
        Self {
            view,
            ty: ByAddress(ty),
            inner_type: None,
            resolved_struct_body: None,
        }
    }
}

impl<'env> Executable<'env> for LowerType<'env> {
    type Output = ir::Type<'env>;

    fn execute(
        self,
        executor: &Executor<'env>,
        ctx: &mut ExecutionCtx<'env>,
    ) -> Result<Self::Output, Continuation<'env>> {
        let target = self.view.target();

        return Ok(match &self.ty.kind {
            TypeKind::IntegerLiteral(_) => ir::Type::Void,
            TypeKind::IntegerLiteralInRange(min, max) => {
                let (bits, sign) =
                    bits_and_sign_for_invisible_integer_in_range(min, max).map_err(|_| {
                        ErrorDiagnostic::new("Integer range too large to represent", self.ty.source)
                    })?;

                ir::Type::I(bits, sign)
            }
            TypeKind::FloatLiteral(_) => ir::Type::Void,
            TypeKind::NullLiteral => ir::Type::Void,
            TypeKind::BooleanLiteral(_) => ir::Type::Void,
            TypeKind::AsciiCharLiteral(_) => ir::Type::Void,
            TypeKind::Boolean => ir::Type::Bool,
            TypeKind::BitInteger(bits, sign) => ir::Type::I(*bits, *sign),
            TypeKind::CInteger(c_integer, sign) => {
                let bytes = target.c_integer_bytes(*c_integer);
                let Some(bits) = IntegerBits::new(bytes.to_bits()) else {
                    return Err(ErrorDiagnostic::new(
                        "C integer is larger that supported",
                        self.ty.source,
                    )
                    .into());
                };

                ir::Type::I(
                    bits,
                    sign.unwrap_or_else(|| target.default_c_integer_sign(*c_integer)),
                )
            }
            TypeKind::SizeInteger(sign) => {
                let bytes = target.size_layout().width;
                let Some(bits) = IntegerBits::new(bytes.to_bits()) else {
                    return Err(ErrorDiagnostic::new(
                        "Size integer is larger that supported",
                        self.ty.source,
                    )
                    .into());
                };

                ir::Type::I(bits, *sign)
            }
            TypeKind::Floating(size) => ir::Type::F(*size),
            TypeKind::Ptr(inner) | TypeKind::Deref(inner) => {
                let Some(lowered_inner) = executor.demand(self.inner_type) else {
                    return suspend!(
                        self.inner_type,
                        executor.request(LowerType::new(self.view, *inner)),
                        ctx
                    );
                };

                ir::Type::Ptr(ctx.alloc(lowered_inner))
            }
            TypeKind::Void | TypeKind::Never => ir::Type::Void,
            TypeKind::FixedArray(_, _) => todo!("LowerType FixedArray"),
            TypeKind::UserDefined(udt) => {
                assert_eq!(udt.args.len(), 0);

                match &udt.rest.kind {
                    TypeHeadRestKind::Struct(structure) => {
                        let Some(resolved_struct_body) = executor.demand(self.resolved_struct_body)
                        else {
                            return suspend!(
                                self.resolved_struct_body,
                                executor.request(ResolveStructureBody::new(self.view, structure)),
                                ctx
                            );
                        };

                        let Some(lowered_struct_type) = executor.demand(self.inner_type) else {
                            return suspend!(
                                self.inner_type,
                                executor.request(LowerTypeStructure::new(
                                    self.view,
                                    structure,
                                    resolved_struct_body
                                )),
                                ctx
                            );
                        };

                        lowered_struct_type
                    }
                    TypeHeadRestKind::Alias(_) => todo!("LowerType for type alias"),
                }
            }
            TypeKind::Polymorph(_) => {
                return Err(ErrorDiagnostic::new(
                    "Cannot lower unspecialized polymorph",
                    self.ty.source,
                )
                .into());
            }
            TypeKind::DirectLabel(_) => ir::Type::Void,
        });
    }
}
