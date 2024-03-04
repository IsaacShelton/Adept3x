use super::{
    builder::Builder, intrinsics::Intrinsics, module::BackendModule, target_data::TargetData,
    value_catalog::ValueCatalog, variable_stack::VariableStack,
};
use crate::ir;
use llvm_sys::{
    prelude::{LLVMBuilderRef, LLVMTypeRef, LLVMValueRef},
    LLVMModule,
};
use std::collections::HashMap;

pub struct Phi2Relocation {
    pub phi: LLVMValueRef,
    pub a: LLVMValueRef,
    pub b: LLVMValueRef,
    pub basicblock_a: usize,
    pub basicblock_b: usize,
}

pub struct StaticVariable {
    pub global: LLVMValueRef,
    pub ty: LLVMTypeRef,
}

pub struct BackendContext<'a> {
    pub backend_module: &'a BackendModule,
    pub ir_module: ir::Module,
    pub builder: Option<Builder>,
    pub value_catalog: Option<ValueCatalog>,
    pub variable_stack: Option<VariableStack>,
    pub func_skeletons: HashMap<ir::FunctionRef, LLVMValueRef>,
    pub global_variables: Vec<LLVMValueRef>,
    pub anon_global_variables: Vec<LLVMValueRef>,
    pub target_data: &'a TargetData,
    pub intrinsics: Intrinsics,
    pub relocations: Vec<Phi2Relocation>,
    pub static_variables: Vec<StaticVariable>,
}

impl<'a> BackendContext<'a> {
    pub unsafe fn new(
        ir_module: &'a ir::Module,
        backend_module: &'a BackendModule,
        target_data: &'a TargetData,
    ) -> Self {
        Self {
            ir_module: ir_module.clone(),
            backend_module,
            builder: None,
            value_catalog: None,
            variable_stack: None,
            func_skeletons: HashMap::new(),
            global_variables: Vec::new(),
            anon_global_variables: Vec::new(),
            target_data,
            intrinsics: Intrinsics::new(&backend_module),
            relocations: Vec::new(),
            static_variables: Vec::new(),
        }
    }
}
