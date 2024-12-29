/*
    ==============================  ir/funcs.rs  ==============================
    Type-safe wrapper to hold IR functions and map ir::FunctionRefs to them.

    Has undefined behavior if FuncRefs are used for multiple maps.
    In practice, this means panicing in debug mode, or out-of-bounds in
    release mode.
    ---------------------------------------------------------------------------
*/

use super::Func;
use crate::{asg, ir, resolve::PolyRecipe};
use append_only_vec::AppendOnlyVec;
use std::{borrow::Borrow, collections::HashMap, sync::RwLock};

pub struct Funcs {
    funcs: AppendOnlyVec<Func>,
    monomorphized: RwLock<HashMap<(asg::FuncRef, PolyRecipe), ir::FuncRef>>,
    jobs: AppendOnlyVec<(asg::FuncRef, PolyRecipe, ir::FuncRef)>,
}

impl Funcs {
    pub fn new() -> Self {
        Self {
            funcs: AppendOnlyVec::new(),
            monomorphized: RwLock::new(HashMap::new()),
            jobs: AppendOnlyVec::new(),
        }
    }

    pub fn insert(&self, func_ref: asg::FuncRef, function: Func) -> ir::FuncRef {
        let index = self.funcs.len();
        self.funcs.push(function);
        let ir_func_ref = ir::FuncRef { index };
        self.monomorphized
            .write()
            .unwrap()
            .insert((func_ref, PolyRecipe::default()), ir_func_ref);
        ir_func_ref
    }

    pub fn translate<E>(
        &self,
        func_ref: asg::FuncRef,
        poly_recipe: impl Borrow<PolyRecipe>,
        monomorphize: impl Fn() -> Result<ir::FuncRef, E>,
    ) -> Result<ir::FuncRef, E> {
        let key = (func_ref, poly_recipe.borrow().clone());

        if let Some(found) = self.monomorphized.read().unwrap().get(&key) {
            return Ok(*found);
        }

        let ir_func_ref = monomorphize()?;

        self.monomorphized.write().unwrap().insert(key, ir_func_ref);

        self.jobs
            .push((func_ref, poly_recipe.borrow().clone(), ir_func_ref));

        Ok(ir_func_ref)
    }

    pub fn get(&self, key: ir::FuncRef) -> &Func {
        &self.funcs[key.index]
    }

    pub fn get_mut(&mut self, key: ir::FuncRef) -> &mut Func {
        &mut self.funcs[key.index]
    }

    pub fn values(&self) -> impl Iterator<Item = &Func> {
        self.funcs.iter()
    }

    pub fn iter(&self) -> impl Iterator<Item = (ir::FuncRef, &Func)> {
        self.funcs
            .iter()
            .enumerate()
            .map(|(index, function)| (ir::FuncRef { index }, function))
    }

    pub fn monomorphized<'a>(
        &'a self,
    ) -> impl Iterator<Item = &'a (asg::FuncRef, PolyRecipe, ir::FuncRef)> {
        Monomorphized {
            vec: &self.jobs,
            i: 0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Monomorphized<'a> {
    vec: &'a AppendOnlyVec<(asg::FuncRef, PolyRecipe, ir::FuncRef)>,
    i: usize,
}

impl<'a> Iterator for Monomorphized<'a> {
    type Item = &'a (asg::FuncRef, PolyRecipe, ir::FuncRef);

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
