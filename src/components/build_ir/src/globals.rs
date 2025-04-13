use arena::{Arena, LockFreeArena};
use std::{collections::HashMap, sync::RwLock};

#[derive(Debug, Default)]
pub struct Globals {
    globals: LockFreeArena<ir::GlobalId, ir::Global>,
    lowered: RwLock<HashMap<asg::GlobalRef, ir::GlobalRef>>,
}

impl Globals {
    pub fn build(self) -> Arena<ir::GlobalId, ir::Global> {
        self.globals.into_arena()
    }

    pub fn translate(&self, key: asg::GlobalRef) -> ir::GlobalRef {
        *self
            .lowered
            .read()
            .unwrap()
            .get(&key)
            .expect("global variable to have already been lowered")
    }

    pub fn insert(&self, asg_global_ref: asg::GlobalRef, global: ir::Global) -> ir::GlobalRef {
        let ir_global_ref = self.globals.alloc(global);

        self.lowered
            .write()
            .unwrap()
            .insert(asg_global_ref, ir_global_ref);

        ir_global_ref
    }
}
