use super::{Params, Type, TypeParams, UserDefinedType};
use attributes::{SymbolOwnership, Tag};
use indexmap::IndexMap;
use source_files::Source;

#[derive(Clone, Debug)]
pub struct FuncHead<'env> {
    pub name: &'env str,
    pub type_params: TypeParams,
    pub params: Params<'env>,
    pub return_type: &'env Type<'env>,
    pub impl_params: ImplParams<'env>,
    pub source: Source,
    pub metadata: FuncMetadata,
}

#[derive(Clone, Debug)]
pub struct FuncMetadata {
    pub abi: TargetAbi,
    pub ownership: SymbolOwnership,
    pub tag: Option<Tag>,
}

#[derive(Copy, Clone, Debug)]
pub enum TargetAbi {
    Abstract,
    C,
}

#[derive(Clone, Debug)]
pub struct ImplParams<'env> {
    pub params: IndexMap<&'env str, &'env UserDefinedType<'env>>,
}
