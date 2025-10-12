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
    build_llvm_ir::{
        builder::Builder,
        ctx::{BackendCtx, FunctionSkeleton},
    },
    ir,
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

pub struct ParamValueConstructionCtx<'a, 'env: 'a> {
    pub builder: &'a mut Builder<'env>,
    pub ctx: &'a BackendCtx<'a, 'env>,
    pub skeleton: &'a FunctionSkeleton<'env>,
    pub param_range: ParamRange,
    pub ir_param_type: &'env ir::Type<'env>,
    pub alloca_point: LLVMValueRef,
}
