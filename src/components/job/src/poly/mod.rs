mod catalog;
mod recipe;
mod type_matcher;

use crate::repr::Type;
pub use catalog::*;
use derive_more::IsVariant;
pub use recipe::*;
pub use type_matcher::*;

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, IsVariant)]
pub enum PolyValue<'env> {
    Type(&'env Type<'env>),
}
