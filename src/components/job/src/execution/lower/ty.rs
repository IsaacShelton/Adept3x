use crate::{
    Continuation, Executable, ExecutionCtx, Executor, Suspend, ir,
    module_graph::ModuleView,
    repr::{Compiler, Type, TypeKind},
    target_layout::{TargetLayout, TypeLayout},
};
use by_address::ByAddress;
use data_units::ByteUnits;
use derivative::Derivative;
use diagnostics::ErrorDiagnostic;
use primitives::{CInteger, IntegerBits, IntegerSign};
use target::Target;

#[derive(Clone, Derivative)]
#[derivative(Debug, PartialEq, Eq, Hash)]
pub struct LowerType<'env> {
    view: ModuleView<'env>,

    #[derivative(Debug = "ignore")]
    compiler: ByAddress<&'env Compiler<'env>>,

    ty: ByAddress<&'env Type<'env>>,

    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    inner_type: Suspend<'env, ir::Type<'env>>,
}

impl<'env> LowerType<'env> {
    pub fn new(
        view: ModuleView<'env>,
        compiler: &'env Compiler<'env>,
        ty: &'env Type<'env>,
    ) -> Self {
        Self {
            view,
            compiler: ByAddress(compiler),
            ty: ByAddress(ty),
            inner_type: None,
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
        let target = self.compiler.target(self.view.graph);

        return Ok(match &self.ty.kind {
            TypeKind::IntegerLiteral(_) => {
                return Err(ErrorDiagnostic::new(
                    "Cannot lower unspecialized integer literal",
                    self.ty.source,
                )
                .into());
            }
            TypeKind::IntegerLiteralInRange(..) => {
                return Err(ErrorDiagnostic::new(
                    "Cannot lower unspecialized integer literal in range",
                    self.ty.source,
                )
                .into());
            }
            TypeKind::FloatLiteral(_) => {
                return Err(ErrorDiagnostic::new(
                    "Cannot lower unspecialized float literal",
                    self.ty.source,
                )
                .into());
            }
            TypeKind::NullLiteral => {
                return Err(ErrorDiagnostic::new(
                    "Cannot lower unspecialized null literal",
                    self.ty.source,
                )
                .into());
            }
            TypeKind::BooleanLiteral(_) => {
                return Err(ErrorDiagnostic::new(
                    "Cannot lower unspecialized boolean literal",
                    self.ty.source,
                )
                .into());
            }
            TypeKind::AsciiCharLiteral(_) => {
                return Err(ErrorDiagnostic::new(
                    "Cannot lower unspecialized ASCII character literal",
                    self.ty.source,
                )
                .into());
            }
            TypeKind::Boolean => ir::Type::Bool,
            TypeKind::BitInteger(bits, sign) => ir::Type::I(*bits, *sign),
            TypeKind::CInteger(integer, sign) => lower_c_integer(&target, *integer, *sign),
            TypeKind::SizeInteger(sign) => {
                let layout = target.size_layout();
                assert_eq!(layout, TypeLayout::basic(ByteUnits::of(8)));
                ir::Type::I(IntegerBits::Bits64, *sign)
            }
            TypeKind::Floating(size) => ir::Type::F(*size),
            TypeKind::Ptr(inner) | TypeKind::Deref(inner, _) => {
                let Some(lowered_inner) = executor.demand(self.inner_type) else {
                    return suspend!(
                        self.inner_type,
                        executor.request(LowerType::new(self.view, &self.compiler, *inner)),
                        ctx
                    );
                };

                ir::Type::Ptr(ctx.alloc(lowered_inner))
            }
            TypeKind::Void | TypeKind::Never => ir::Type::Void,
            TypeKind::FixedArray(_, _) => todo!("LowerType FixedArray"),
            TypeKind::UserDefined(_) => todo!("LowerType UserDefined"),
            TypeKind::Polymorph(_) => {
                return Err(ErrorDiagnostic::new("Cannot lower polymorph", self.ty.source).into());
            }
            TypeKind::DirectLabel(_) => {
                return Err(
                    ErrorDiagnostic::new("Cannot lower direct label type", self.ty.source).into(),
                );
            }
        });
    }
}

fn lower_c_integer<'env>(
    target: &Target,
    c_integer: CInteger,
    sign: Option<IntegerSign>,
) -> ir::Type<'env> {
    let sign = sign.unwrap_or_else(|| target.default_c_integer_sign(c_integer));
    let bits = IntegerBits::new(target.c_integer_bytes(c_integer).to_bits())
        .expect("c integer to be representable with primitive integers");

    ir::Type::I(bits, sign)
}
