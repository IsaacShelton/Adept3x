mod coerce_and_expand;
mod direct;
mod expand;
mod ignore;
mod inalloca;
mod indirect;
mod value;

pub use self::value::ParamValue;
use super::params_mapping::ParamRange;
use crate::{
    builder::Builder,
    ctx::{BackendCtx, FunctionSkeleton},
};
use llvm_sys::prelude::LLVMValueRef;

pub struct ParamValues {
    values: Vec<ParamValue>,
}

impl ParamValues {
    pub fn new() -> Self {
        Self {
            values: Vec::<ParamValue>::with_capacity(16),
        }
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
