use crate::{
    resolved::{self, EnumRef, StructureRef, TypeAliasRef},
    workspace::fs::FsNodeId,
};

#[derive(Clone, Debug)]
pub enum FuncJob {
    Regular(FsNodeId, usize, resolved::FunctionRef),
}

#[derive(Clone, Debug)]
pub struct TypeJob {
    pub physical_file_id: FsNodeId,
    pub type_aliases: Vec<TypeAliasRef>,
    pub structures: Vec<StructureRef>,
    pub enums: Vec<EnumRef>,
}