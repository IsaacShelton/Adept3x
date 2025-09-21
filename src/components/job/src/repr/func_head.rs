use super::{Params, TypeParams};
use crate::{
    module_graph::ModuleView,
    repr::{UnaliasedType, UnaliasedUserDefinedType},
};
use attributes::{SymbolOwnership, Tag};
use derive_more::IsVariant;
use indexmap::IndexMap;
use source_files::Source;

#[derive(Clone, Debug)]
pub struct FuncHead<'env> {
    pub name: &'env str,
    pub type_params: TypeParams,
    pub params: Params<'env>,
    pub return_type: UnaliasedType<'env>,
    pub impl_params: ImplParams<'env>,
    pub source: Source,
    pub metadata: FuncMetadata,
    pub view: &'env ModuleView<'env>,
}

#[derive(Clone, Debug)]
pub struct FuncMetadata {
    pub abi: TargetAbi,
    pub ownership: SymbolOwnership,
    pub tag: Option<Tag>,
}

#[derive(Copy, Clone, Debug, IsVariant)]
pub enum TargetAbi {
    Abstract,
    C,
}

#[derive(Clone, Debug, Default)]
pub struct ImplParams<'env> {
    pub params: IndexMap<&'env str, UnaliasedUserDefinedType<'env>>,
}
