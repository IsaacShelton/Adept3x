/*
    ==============================  ir/funcs.rs  ==============================
    Type-safe wrapper to hold IR functions and map ir::FunctionRefs to them.

    Has undefined behavior if FuncRefs are used for multiple maps.
    In practice, this means panicing in debug mode, or out-of-bounds in
    release mode.
    ---------------------------------------------------------------------------
*/

use super::Func;
use crate::{asg, resolve::PolyRecipe};
use append_only_vec::AppendOnlyVec;
use std::{borrow::Borrow, collections::HashMap, sync::RwLock};

pub struct Funcs {
    functions: AppendOnlyVec<Func>,
    monomorphized: RwLock<HashMap<(asg::FuncRef, PolyRecipe), FuncRef>>,
    jobs: AppendOnlyVec<(asg::FuncRef, PolyRecipe, FuncRef)>,
}

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub struct FuncRef {
    index: usize,
}

impl Funcs {
    pub fn new() -> Self {
        Self {
            functions: AppendOnlyVec::new(),
            monomorphized: RwLock::new(HashMap::new()),
            jobs: AppendOnlyVec::new(),
        }
    }

    pub fn insert(&self, func_ref: asg::FuncRef, function: Func) -> FuncRef {
        let index = self.functions.len();
        self.functions.push(function);
        let ir_function_ref = FuncRef { index };
        self.monomorphized
            .write()
            .unwrap()
            .insert((func_ref, PolyRecipe::default()), ir_function_ref);
        ir_function_ref
    }

    pub fn translate<E>(
        &self,
        func_ref: asg::FuncRef,
        poly_recipe: impl Borrow<PolyRecipe>,
        monomorphize: impl Fn() -> Result<FuncRef, E>,
    ) -> Result<FuncRef, E> {
        let key = (func_ref, poly_recipe.borrow().clone());

        if let Some(found) = self.monomorphized.read().unwrap().get(&key) {
            return Ok(*found);
        }

        let function_ref = monomorphize()?;

        self.monomorphized
            .write()
            .unwrap()
            .insert(key, function_ref);

        self.jobs
            .push((func_ref, poly_recipe.borrow().clone(), function_ref));

        Ok(function_ref)
    }

    pub fn get(&self, key: FuncRef) -> &Func {
        &self.functions[key.index]
    }

    pub fn get_mut(&mut self, key: FuncRef) -> &mut Func {
        &mut self.functions[key.index]
    }

    pub fn values(&self) -> impl Iterator<Item = &Func> {
        self.functions.iter()
    }

    pub fn iter(&self) -> impl Iterator<Item = (FuncRef, &Func)> {
        self.functions
            .iter()
            .enumerate()
            .map(|(index, function)| (FuncRef { index }, function))
    }

    pub fn monomorphized<'a>(
        &'a self,
    ) -> impl Iterator<Item = &'a (asg::FuncRef, PolyRecipe, FuncRef)> {
        Monomorphized {
            vec: &self.jobs,
            i: 0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Monomorphized<'a> {
    vec: &'a AppendOnlyVec<(asg::FuncRef, PolyRecipe, FuncRef)>,
    i: usize,
}

impl<'a> Iterator for Monomorphized<'a> {
    type Item = &'a (asg::FuncRef, PolyRecipe, FuncRef);

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
