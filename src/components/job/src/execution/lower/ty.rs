use crate::{
    Continuation, Executable, ExecutionCtx, Executor, Suspend, ir,
    module_graph::ModuleView,
    repr::{Compiler, Type, TypeKind},
};
use by_address::ByAddress;
use data_units::ByteUnits;
use derivative::Derivative;
use diagnostics::ErrorDiagnostic;
use primitives::{CInteger, FloatSize, IntegerSign};
use target::{Target, TargetOsExt};
use target_layout::{TargetLayout, TypeLayout};

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
        use primitives::{IntegerBits as Bits, IntegerSign as Sign};

        let target = self.compiler.target(self.view.graph);

        return Ok(match &self.ty.kind {
            TypeKind::IntegerLiteral(_) => {
                return Err(ErrorDiagnostic::new(
                    "Cannot lower unspecialized integer type",
                    self.ty.source,
                )
                .into());
            }
            TypeKind::FloatLiteral(_) => {
                return Err(ErrorDiagnostic::new(
                    "Cannot lower unspecialized float type",
                    self.ty.source,
                )
                .into());
            }
            TypeKind::BooleanLiteral(_) => {
                return Err(ErrorDiagnostic::new(
                    "Cannot lower unspecialized boolean type",
                    self.ty.source,
                )
                .into());
            }
            TypeKind::Boolean => ir::Type::Bool,
            TypeKind::BitInteger(bits, sign) => match (bits, sign) {
                (Bits::Bits8, Sign::Signed) => ir::Type::S8,
                (Bits::Bits8, Sign::Unsigned) => ir::Type::U8,
                (Bits::Bits16, Sign::Signed) => ir::Type::S16,
                (Bits::Bits16, Sign::Unsigned) => ir::Type::U16,
                (Bits::Bits32, Sign::Signed) => ir::Type::S32,
                (Bits::Bits32, Sign::Unsigned) => ir::Type::U32,
                (Bits::Bits64, Sign::Signed) => ir::Type::S64,
                (Bits::Bits64, Sign::Unsigned) => ir::Type::U64,
            },
            TypeKind::CInteger(integer, sign) => lower_c_integer(&target, *integer, *sign),
            TypeKind::SizeInteger(sign) => {
                let layout = target.size_layout();
                assert_eq!(layout, TypeLayout::basic(ByteUnits::of(8)));

                match sign {
                    Sign::Signed => ir::Type::S64,
                    Sign::Unsigned => ir::Type::U64,
                }
            }
            TypeKind::Floating(size) => match size {
                FloatSize::Bits32 => ir::Type::F32,
                FloatSize::Bits64 => ir::Type::F64,
            },
            TypeKind::Ptr(inner) => {
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
    integer: CInteger,
    sign: Option<IntegerSign>,
) -> ir::Type<'env> {
    let sign = sign.unwrap_or_else(|| target.default_c_integer_sign(integer));

    match (integer, sign) {
        (CInteger::Char, IntegerSign::Signed) => ir::Type::S8,
        (CInteger::Char, IntegerSign::Unsigned) => ir::Type::U8,
        (CInteger::Short, IntegerSign::Signed) => ir::Type::S16,
        (CInteger::Short, IntegerSign::Unsigned) => ir::Type::U16,
        (CInteger::Int, IntegerSign::Signed) => ir::Type::S32,
        (CInteger::Int, IntegerSign::Unsigned) => ir::Type::U32,
        (CInteger::Long, IntegerSign::Signed) => {
            if target.os().is_windows() {
                ir::Type::S32
            } else {
                ir::Type::S64
            }
        }
        (CInteger::Long, IntegerSign::Unsigned) => {
            if target.os().is_windows() {
                ir::Type::U32
            } else {
                ir::Type::U64
            }
        }
        (CInteger::LongLong, IntegerSign::Signed) => ir::Type::S64,
        (CInteger::LongLong, IntegerSign::Unsigned) => ir::Type::U64,
    }
}
