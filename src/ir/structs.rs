use super::Structure;
use crate::{asg, resolve::PolyRecipe};
use append_only_vec::AppendOnlyVec;
use std::{collections::HashMap, sync::RwLock};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct StructRef {
    index: usize,
}

#[derive(Debug)]
pub struct Structs {
    structures: AppendOnlyVec<Structure>,
    monomorphized: RwLock<HashMap<(asg::StructRef, PolyRecipe), StructRef>>,
    jobs: AppendOnlyVec<(asg::StructRef, PolyRecipe, StructRef)>,
}

impl Structs {
    pub fn new() -> Self {
        Self {
            structures: AppendOnlyVec::new(),
            monomorphized: RwLock::new(HashMap::default()),
            jobs: AppendOnlyVec::new(),
        }
    }

    pub fn insert(
        &self,
        struct_ref: asg::StructRef,
        structure: Structure,
        poly_recipe: PolyRecipe,
    ) -> StructRef {
        let structure_ref = StructRef {
            index: self.structures.push(structure),
        };

        let key = (struct_ref, poly_recipe);
        self.monomorphized
            .write()
            .unwrap()
            .insert(key, structure_ref);

        structure_ref
    }

    pub fn translate<E>(
        &self,
        struct_ref: asg::StructRef,
        poly_recipe: PolyRecipe,
        monomorphize: impl Fn(PolyRecipe) -> Result<StructRef, E>,
    ) -> Result<StructRef, E> {
        let key = (struct_ref, poly_recipe);

        if let Some(found) = self.monomorphized.read().unwrap().get(&key) {
            return Ok(*found);
        }

        let poly_recipe = key.1.clone();
        let function_ref = monomorphize(poly_recipe.clone())?;

        self.monomorphized
            .write()
            .unwrap()
            .insert(key, function_ref);

        self.jobs
            .push((struct_ref, poly_recipe.clone(), function_ref));

        Ok(function_ref)
    }

    pub fn get(&self, key: StructRef) -> &Structure {
        &self.structures[key.index]
    }

    pub fn monomorphized<'a>(
        &'a self,
    ) -> impl Iterator<Item = &'a (asg::StructRef, PolyRecipe, StructRef)> {
        Monomorphized {
            vec: &self.jobs,
            i: 0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Monomorphized<'a> {
    vec: &'a AppendOnlyVec<(asg::StructRef, PolyRecipe, StructRef)>,
    i: usize,
}

impl<'a> Iterator for Monomorphized<'a> {
    type Item = &'a (asg::StructRef, PolyRecipe, StructRef);

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
