use super::{abi_type::ABIType, arch::Arch, cxx::Itanium};
use crate::{
    ir,
    llvm_backend::{ctx::BackendCtx, error::BackendError},
};

#[derive(Clone, Debug)]
pub struct ABIFunction {
    pub parameter_types: Vec<ABIType>,
    pub return_type: ABIType,
}

impl ABIFunction {
    pub fn new(
        backend_ctx: &BackendCtx<'_>,
        arch: Arch,
        parameter_types: &[&ir::Type],
        return_type: &ir::Type,
        is_variadic: bool,
    ) -> Result<Self, BackendError> {
        let info = arch.core_info();

        match arch {
            Arch::X86_64(_abi) => todo!(),
            Arch::AARCH64(abi) => {
                let itanium = Itanium {
                    target_info: info.target_info,
                    type_info_manager: info.type_info_manager,
                };

                abi.compute_info(
                    backend_ctx,
                    itanium,
                    parameter_types,
                    return_type,
                    is_variadic,
                )
            }
        }
    }
}
