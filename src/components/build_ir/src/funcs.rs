use crate::error::{LowerError, LowerErrorKind};
use append_only_vec::AppendOnlyVec;
use asg::PolyRecipe;
use attributes::Tag;
use std::{borrow::Borrow, cell::OnceCell, collections::HashMap, sync::RwLock};

#[derive(Default)]
pub struct Funcs {
    funcs: AppendOnlyVec<ir::Func>,
    monomorphized: RwLock<HashMap<(asg::FuncRef, PolyRecipe), ir::FuncRef>>,
    jobs: AppendOnlyVec<(asg::FuncRef, PolyRecipe, ir::FuncRef)>,
    interpreter_entry_point: OnceCell<ir::FuncRef>,
}

impl Funcs {
    pub fn build(self) -> ir::Funcs {
        ir::Funcs::new(self.funcs.into_iter().collect())
    }

    pub fn insert(&self, func_ref: asg::FuncRef, function: ir::Func) -> ir::FuncRef {
        let index = self.funcs.len();
        self.funcs.push(function);
        let ir_func_ref = ir::FuncRef::new(index);
        self.monomorphized
            .write()
            .unwrap()
            .insert((func_ref, PolyRecipe::default()), ir_func_ref);
        ir_func_ref
    }

    pub fn translate(
        &self,
        asg: &asg::Asg,
        func_ref: asg::FuncRef,
        poly_recipe: impl Borrow<PolyRecipe>,
        monomorphize: impl Fn() -> Result<ir::FuncRef, LowerError>,
    ) -> Result<ir::FuncRef, LowerError> {
        let key = (func_ref, poly_recipe.borrow().clone());

        if let Some(found) = self.monomorphized.read().unwrap().get(&key) {
            return Ok(*found);
        }

        let ir_func_ref = monomorphize()?;
        self.monomorphized.write().unwrap().insert(key, ir_func_ref);

        self.jobs
            .push((func_ref, poly_recipe.borrow().clone(), ir_func_ref));

        let asg_func = &asg.funcs[func_ref];

        if asg_func.tag == Some(Tag::InterpreterEntryPoint) {
            self.interpreter_entry_point.set(ir_func_ref).map_err(|_| {
                LowerErrorKind::Other {
                    message: "Cannot have multiple entry points".into(),
                }
                .at(asg_func.source)
            })?;
        }

        Ok(ir_func_ref)
    }

    pub fn get_mut(&mut self, key: ir::FuncRef) -> &mut ir::Func {
        &mut self.funcs[key.get()]
    }

    pub fn monomorphized<'a>(
        &'a self,
    ) -> impl Iterator<Item = &'a (asg::FuncRef, PolyRecipe, ir::FuncRef)> {
        Monomorphized {
            vec: &self.jobs,
            i: 0,
        }
    }

    pub fn interpreter_entry_point(&self) -> Option<ir::FuncRef> {
        self.interpreter_entry_point.get().copied()
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
