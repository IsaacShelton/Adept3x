use crate::{
    asg::{self, EnumRef, StructRef, TraitRef, TypeAliasRef},
    workspace::fs::FsNodeId,
};

#[derive(Clone, Debug)]
pub enum FuncJob {
    Regular(FsNodeId, usize, asg::FuncRef),
    Impling(FsNodeId, usize, usize, asg::FuncRef),
}

#[derive(Clone, Debug)]
pub struct TypeJob {
    pub physical_file_id: FsNodeId,
    pub type_aliases: Vec<TypeAliasRef>,
    pub traits: Vec<TraitRef>,
    pub structures: Vec<StructRef>,
    pub enums: Vec<EnumRef>,
}
