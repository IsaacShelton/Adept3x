use super::{
    abi::{
        abi_function::ABIFunction,
        arch::{aarch64, Arch},
    },
    builder::Builder,
    functions::function_type::FunctionType,
    intrinsics::Intrinsics,
    module::BackendModule,
    target_data::TargetData,
};
use crate::{
    diagnostics::Diagnostics,
    ir,
    resolved::{self, StructureRef},
    target_info::type_layout::TypeLayoutCache,
};
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
pub struct StructureCache {
    pub cache: OnceMap<StructureRef, LLVMTypeRef>,
}

#[derive(Debug)]
pub struct ToBackendTypeCtx<'a> {
    pub structure_cache: &'a StructureCache,
    pub ir_module: &'a ir::Module,
    pub visited: RefCell<HashSet<StructureRef>>,
}

impl<'a> From<&'a BackendCtx<'a>> for ToBackendTypeCtx<'a> {
    fn from(value: &'a BackendCtx<'a>) -> Self {
        value.for_making_type()
    }
}

pub struct FunctionSkeleton {
    pub function: LLVMValueRef,
    pub abi_function: Option<ABIFunction>,
    pub function_type: FunctionType,
    pub ir_function_ref: ir::FunctionRef,
}

pub struct BackendCtx<'a> {
    pub backend_module: &'a BackendModule,
    pub ir_module: &'a ir::Module,
    pub builder: Option<Builder>,
    pub func_skeletons: HashMap<ir::FunctionRef, FunctionSkeleton>,
    pub globals: HashMap<ir::GlobalRef, LLVMValueRef>,
    pub anon_global_variables: Vec<LLVMValueRef>,
    pub target_data: &'a TargetData,
    pub intrinsics: Intrinsics,
    pub relocations: Vec<Phi2Relocation>,
    pub static_variables: Vec<StaticVariable>,
    pub structure_cache: StructureCache,
    pub type_layout_cache: TypeLayoutCache<'a>,
    pub arch: Arch,
}

impl<'a> BackendCtx<'a> {
    pub unsafe fn new(
        ir_module: &'a ir::Module,
        backend_module: &'a BackendModule,
        target_data: &'a TargetData,
        resolved_ast: &'a resolved::Ast,
        diagnostics: &'a Diagnostics,
    ) -> Self {
        let type_layout_cache = TypeLayoutCache::new(
            &ir_module.target_info,
            &ir_module.structures,
            resolved_ast,
            diagnostics,
        );

        Self {
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
            structure_cache: Default::default(),
            type_layout_cache,
            arch: Arch::AARCH64(aarch64::AARCH64 {
                variant: aarch64::Variant::DarwinPCS,
                is_cxx_mode: false,
            }),
        }
    }

    pub fn for_making_type(&'a self) -> ToBackendTypeCtx<'a> {
        ToBackendTypeCtx {
            structure_cache: &self.structure_cache,
            ir_module: self.ir_module,
            visited: RefCell::new(HashSet::default()),
        }
    }
}
