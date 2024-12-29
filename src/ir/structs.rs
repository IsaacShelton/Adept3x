use super::Struct;
use crate::{asg, ir, resolve::PolyRecipe};
use append_only_vec::AppendOnlyVec;
use std::{collections::HashMap, sync::RwLock};

#[derive(Debug)]
pub struct Structs {
    structs: AppendOnlyVec<Struct>,
    monomorphized: RwLock<HashMap<(asg::StructRef, PolyRecipe), ir::StructRef>>,
    jobs: AppendOnlyVec<(asg::StructRef, PolyRecipe, ir::StructRef)>,
}

impl Structs {
    pub fn new() -> Self {
        Self {
            structs: AppendOnlyVec::new(),
            monomorphized: RwLock::new(HashMap::default()),
            jobs: AppendOnlyVec::new(),
        }
    }

    pub fn insert(
        &self,
        struct_ref: asg::StructRef,
        structure: Struct,
        poly_recipe: PolyRecipe,
    ) -> ir::StructRef {
        let ir_struct_ref = ir::StructRef {
            index: self.structs.push(structure),
        };

        let key = (struct_ref, poly_recipe);
        self.monomorphized
            .write()
            .unwrap()
            .insert(key, ir_struct_ref);

        ir_struct_ref
    }

    pub fn translate<E>(
        &self,
        struct_ref: asg::StructRef,
        poly_recipe: PolyRecipe,
        monomorphize: impl Fn(PolyRecipe) -> Result<ir::StructRef, E>,
    ) -> Result<ir::StructRef, E> {
        let key = (struct_ref, poly_recipe);

        if let Some(found) = self.monomorphized.read().unwrap().get(&key) {
            return Ok(*found);
        }

        let poly_recipe = key.1.clone();
        let func_ref = monomorphize(poly_recipe.clone())?;

        self.monomorphized.write().unwrap().insert(key, func_ref);

        self.jobs.push((struct_ref, poly_recipe.clone(), func_ref));

        Ok(func_ref)
    }

    pub fn get(&self, key: ir::StructRef) -> &Struct {
        &self.structs[key.index]
    }

    pub fn monomorphized<'a>(
        &'a self,
    ) -> impl Iterator<Item = &'a (asg::StructRef, PolyRecipe, ir::StructRef)> {
        Monomorphized {
            vec: &self.jobs,
            i: 0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Monomorphized<'a> {
    vec: &'a AppendOnlyVec<(asg::StructRef, PolyRecipe, ir::StructRef)>,
    i: usize,
}

impl<'a> Iterator for Monomorphized<'a> {
    type Item = &'a (asg::StructRef, PolyRecipe, ir::StructRef);

    fn next(&mut self) -> Option<Self::Item> {
        if self.i < self.vec.len() {
            let item = &self.vec[self.i];
            self.i += 1;
            Some(item)
        } else {
            None
        }
    }
}
