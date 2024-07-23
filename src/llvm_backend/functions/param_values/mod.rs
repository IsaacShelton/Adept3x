mod coerce_and_expand;
mod direct;
mod expand;
mod helpers;
mod ignore;
mod inalloca;
mod indirect;
mod value;

use super::params_mapping::ParamRange;
use crate::{
    ir,
    llvm_backend::{
        builder::Builder,
        ctx::{BackendCtx, FunctionSkeleton},
    },
};
use llvm_sys::prelude::LLVMValueRef;

pub use self::value::ParamValue;

pub struct ParamValues {
    values: Vec<ParamValue>,
}

impl ParamValues {
    pub fn new() -> Self {
        Self {
            values: Vec::<ParamValue>::with_capacity(16),
        }
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = &ParamValue> {
        self.values.iter()
    }

    pub fn get(&self, index: usize) -> Option<&ParamValue> {
        self.values.get(index)
    }
}

pub struct ParamValueConstructionCtx<'a> {
    pub builder: &'a Builder,
    pub ctx: &'a BackendCtx<'a>,
    pub skeleton: &'a FunctionSkeleton,
    pub param_range: ParamRange,
    pub ir_param_type: &'a ir::Type,
    pub alloca_point: LLVMValueRef,
}
