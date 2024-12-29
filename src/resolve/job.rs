use crate::{
    asg::{self, EnumRef, StructureRef, TraitRef, TypeAliasRef},
    workspace::fs::FsNodeId,
};

#[derive(Clone, Debug)]
pub enum FuncJob {
    Regular(FsNodeId, usize, asg::FunctionRef),
    Impling(FsNodeId, usize, usize, asg::FunctionRef),
}

#[derive(Clone, Debug)]
pub struct TypeJob {
    pub physical_file_id: FsNodeId,
    pub type_aliases: Vec<TypeAliasRef>,
    pub traits: Vec<TraitRef>,
    pub structures: Vec<StructureRef>,
    pub enums: Vec<EnumRef>,
}
