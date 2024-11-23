/*
    ============================  ir/functions.rs  ============================
    Type-safe wrapper to hold IR functions and map ir::FunctionRefs to them.

    Has undefined behavior if FunctionRefs are used for multiple maps.
    In practice, this means panicing in debug mode, or out-of-bounds in
    release mode.
    ---------------------------------------------------------------------------
*/

use super::Function;
use crate::resolved::{self, PolyRecipe};
use append_only_vec::AppendOnlyVec;
use std::{borrow::Borrow, collections::HashMap, sync::RwLock};

pub struct Functions {
    functions: AppendOnlyVec<Function>,
    monomorphized: RwLock<HashMap<(resolved::FunctionRef, PolyRecipe), FunctionRef>>,
}

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub struct FunctionRef {
    index: usize,
}

impl Functions {
    pub fn new() -> Self {
        Self {
            functions: AppendOnlyVec::new(),
            monomorphized: RwLock::new(HashMap::new()),
        }
    }

    pub fn insert(
        &mut self,
        resolved_function_ref: resolved::FunctionRef,
        function: Function,
    ) -> FunctionRef {
        let index = self.functions.len();
        self.functions.push(function);
        let ir_function_ref = FunctionRef { index };
        self.monomorphized.write().unwrap().insert(
            (resolved_function_ref, PolyRecipe::default()),
            ir_function_ref,
        );
        ir_function_ref
    }

    pub fn translate(
        &self,
        resolved_function_ref: resolved::FunctionRef,
        poly_recipe: impl Borrow<PolyRecipe>,
    ) -> FunctionRef {
        if let Some(found) = self
            .monomorphized
            .read()
            .unwrap()
            .get(&(resolved_function_ref, poly_recipe.borrow().clone()))
        {
            return *found;
        }

        let function_ref = FunctionRef {
            index: self.functions.push(todo!(
                "generate function head for polymorphic function monomorphization"
            )),
        };

        self.monomorphized.write().unwrap().insert(
            (resolved_function_ref, poly_recipe.borrow().clone()),
            function_ref,
        );

        function_ref
    }

    pub fn get(&self, key: FunctionRef) -> &Function {
        &self.functions[key.index]
    }

    pub fn get_mut(&mut self, key: FunctionRef) -> &mut Function {
        &mut self.functions[key.index]
    }

    pub fn values(&self) -> impl Iterator<Item = &Function> {
        self.functions.iter()
    }

    pub fn iter(&self) -> impl Iterator<Item = (FunctionRef, &Function)> {
        self.functions
            .iter()
            .enumerate()
            .map(|(index, function)| (FunctionRef { index }, function))
    }
}
