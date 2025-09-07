use super::{
    abi::{abi_function::ABIFunction, arch::Arch},
    builder::Builder,
    functions::function_type::FunctionType,
    intrinsics::Intrinsics,
    module::BackendModule,
    target_data::TargetData,
};
use crate::{ExecutionCtx, ir, module_graph::ModuleGraphMeta, target_layout::TypeLayoutCache};
use data_units::ByteUnits;
use diagnostics::{Diagnostics, ErrorDiagnostic};
use llvm_sys::prelude::{LLVMTypeRef, LLVMValueRef};
use once_map::unsync::OnceMap;
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
};

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

#[derive(Debug, Default)]
pub struct StructCache<'env> {
    pub cache: OnceMap<ir::StructRef<'env>, LLVMTypeRef>,
}

#[derive(Debug)]
pub struct ToBackendTypeCtx<'a, 'env: 'a> {
    pub struct_cache: &'a StructCache<'env>,
    pub ir_module: &'a ir::Ir<'env>,
    pub visited: RefCell<HashSet<ir::StructRef<'env>>>,
}

impl<'a, 'env> From<&'a BackendCtx<'a, 'env>> for ToBackendTypeCtx<'a, 'env> {
    fn from(value: &'a BackendCtx<'a, 'env>) -> Self {
        value.for_making_type()
    }
}

pub struct FunctionSkeleton<'env> {
    pub function: LLVMValueRef,
    pub abi_function: Option<ABIFunction<'env>>,
    pub function_type: FunctionType,
    pub ir_func_ref: ir::FuncRef<'env>,
    pub max_vector_width: ByteUnits,
}

pub struct BackendCtx<'a, 'env> {
    pub alloc: &'a mut ExecutionCtx<'env>,
    pub backend_module: &'a BackendModule,
    pub ir_module: &'env ir::Ir<'env>,
    pub builder: Option<Builder<'env>>,
    pub func_skeletons: HashMap<ir::FuncRef<'env>, FunctionSkeleton<'env>>,
    pub globals: HashMap<ir::GlobalRef<'env>, LLVMValueRef>,
    pub anon_global_variables: Vec<LLVMValueRef>,
    pub target_data: &'a TargetData,
    pub intrinsics: Intrinsics,
    pub relocations: Vec<Phi2Relocation>,
    pub static_variables: Vec<StaticVariable>,
    pub struct_cache: StructCache<'env>,
    pub type_layout_cache: TypeLayoutCache<'env>,
    pub arch: Arch,
    pub meta: &'a ModuleGraphMeta,
}

impl<'a, 'env> BackendCtx<'a, 'env> {
    pub unsafe fn new(
        alloc: &'a mut ExecutionCtx<'env>,
        ir_module: &'env ir::Ir<'env>,
        meta: &'a ModuleGraphMeta,
        backend_module: &'a BackendModule,
        target_data: &'a TargetData,
        diagnostics: &'env Diagnostics,
    ) -> Result<Self, ErrorDiagnostic> {
        let type_layout_cache =
            TypeLayoutCache::new(meta.target.clone(), &ir_module.structs, diagnostics);

        let arch = Arch::new(&meta.target)
            .ok_or_else(|| ErrorDiagnostic::plain("Target platform is not supported"))?;

        Ok(Self {
            alloc,
            ir_module,
            backend_module,
            builder: None,
            func_skeletons: HashMap::new(),
            globals: HashMap::new(),
            anon_global_variables: Vec::new(),
            target_data,
            intrinsics: Intrinsics::new(backend_module),
            relocations: Vec::new(),
            static_variables: Vec::new(),
            struct_cache: StructCache::default(),
            type_layout_cache,
            arch,
            meta,
        })
    }

    pub fn for_making_type<'b>(&'b self) -> ToBackendTypeCtx<'b, 'env> {
        ToBackendTypeCtx {
            struct_cache: &self.struct_cache,
            ir_module: self.ir_module,
            visited: RefCell::new(HashSet::default()),
        }
    }
}
