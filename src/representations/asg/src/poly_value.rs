use crate::{ImplRef, Type, TypedExpr};
use derive_more::IsVariant;

#[derive(Clone, Debug, Hash, PartialEq, Eq, IsVariant)]
pub enum PolyValue {
    Type(Type),
    Expr(TypedExpr),
    Impl(ImplRef),
    PolyImpl(String),
}
