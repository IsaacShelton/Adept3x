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
    data_units::ByteUnits,
    diagnostics::{Diagnostics, ErrorDiagnostic},
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
    pub ir_module: &'a ir::Module<'a>,
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
    pub max_vector_width: ByteUnits,
}

pub struct BackendCtx<'a> {
    pub backend_module: &'a BackendModule,
    pub ir_module: &'a ir::Module<'a>,
    pub builder: Option<Builder>,
    pub func_skeletons: HashMap<ir::FunctionRef, FunctionSkeleton>,
    pub globals: HashMap<ir::GlobalVarRef, LLVMValueRef>,
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

        #[allow(unused_imports)]
        use crate::llvm_backend::abi::arch::{
            x86_64::{AvxLevel, SysV, SysVOs, X86_64},
            Arch,
        };

        #[allow(unused_assignments)]
        let mut arch = None;

        #[cfg(target_arch = "x86_64")]
        {
            arch = Some(Arch::X86_64(X86_64::SysV(SysV {
                os: SysVOs::Linux,
                avx_level: AvxLevel::None,
            })));
        }

        #[cfg(target_arch = "aarch64")]
        {
            arch = Some(Arch::Aarch64(aarch64::Aarch64 {
                variant: aarch64::Aarch64Variant::DarwinPCS,
                is_cxx_mode: false,
            }));
        }

        // TODO: Add proper error handling
        let Some(arch) = arch else {
            diagnostics.push(ErrorDiagnostic::plain("This platform is not supported"));
            std::process::exit(1);
        };

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
            arch,
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
