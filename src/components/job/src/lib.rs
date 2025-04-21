#[allow(unused_imports)]
use arena::{Arena, ArenaMap, Id, new_id_with_niche};
#[allow(unused_imports)]
use asg::PolyRecipe;
#[allow(unused_imports)]
use std::collections::{HashMap, VecDeque};

// We will probably want something like this:

/*
new_id_with_niche!(NodeId, u64);
new_id_with_niche!(AstFuncId, u64);
new_id_with_niche!(AstStructId, u64);
new_id_with_niche!(AsgFuncId, u64);

pub struct Scheduler {
    queue: VecDeque<NodeId>,
    nodes: Arena<NodeId, Node>,
    build_asg_func_head: ArenaMap<AstFuncId, NodeId>,
    build_asg_func_body: ArenaMap<AstFuncId, NodeId>,
    build_asg_struct: ArenaMap<AstStructId, NodeId>,
    build_ir_func_head: HashMap<(AsgFuncId, PolyRecipe), NodeId>,
    build_ir_func_body: HashMap<(AsgFuncId, PolyRecipe), NodeId>,
}

pub struct Node {
    id: NodeId,
    task: Task,
    edges: Vec<NodeId>,
    in_degree: usize,
}

pub enum Task {
    Running(RunningTask),
    Suspended(SuspendedTask),
    Completed(CompletedTask),
}

pub enum RunningTask {
    BuildAsgFuncHead(ast_workspace::FuncRef),
    BuildAsgFuncBody(ast_workspace::FuncRef, asg::FuncId),
    BuildAsgStruct(ast_workspace::StructRef),
    BuildIrFuncHead(asg::FuncRef, PolyRecipe),
    BuildIrFuncBody(asg::FuncRef, PolyRecipe),
}

pub enum CompletedTask {
    AsgFuncHead(asg::FuncRef),
    AsgFuncBody(asg::FuncRef),
    AsgStruct(asg::StructRef),
    IrFuncHead(ir::FuncRef),
    IrFuncBody(ir::FuncRef),
}
*/
