use arena::{Arena, LockFreeArena};
use asg::PolyRecipe;
use std::{collections::HashMap, sync::RwLock};

#[derive(Debug, Default)]
pub struct Structs {
    structs: LockFreeArena<ir::StructId, ir::Struct>,
    monomorphized: RwLock<HashMap<(asg::StructRef, PolyRecipe), ir::StructRef>>,
}

impl Structs {
    pub fn build(self) -> Arena<ir::StructId, ir::Struct> {
        self.structs.into_arena()
    }

    pub fn insert(
        &self,
        asg_struct_ref: asg::StructRef,
        structure: ir::Struct,
        poly_recipe: PolyRecipe,
    ) -> ir::StructRef {
        let ir_struct_ref = self.structs.alloc(structure);

        let key = (asg_struct_ref, poly_recipe);
        self.monomorphized
            .write()
            .unwrap()
            .insert(key, ir_struct_ref);

        ir_struct_ref
    }

    pub fn translate<E>(
        &self,
        asg_struct_ref: asg::StructRef,
        poly_recipe: PolyRecipe,
        monomorphize: impl Fn(PolyRecipe) -> Result<ir::StructRef, E>,
    ) -> Result<ir::StructRef, E> {
        let key = (asg_struct_ref, poly_recipe);

        if let Some(found) = self.monomorphized.read().unwrap().get(&key) {
            return Ok(*found);
        }

        let poly_recipe = key.1.clone();
        let ir_struct_ref = monomorphize(poly_recipe.clone())?;

        self.monomorphized
            .write()
            .unwrap()
            .insert(key, ir_struct_ref);

        Ok(ir_struct_ref)
    }
}
