use crate::{
    llvm_backend::abi::{abi_function::ABIFunction, abi_type::ABITypeKind},
    target_info::type_layout::TypeLayoutCache,
};
use llvm_sys::{
    core::{LLVMCountStructElementTypes, LLVMGetTypeKind},
    LLVMTypeKind,
};

#[derive(Debug)]
struct Param {
    padding_index: Option<usize>,
    begin_index: usize,
    num_subparams: usize,
}

// Maps IR parameters to LLVM-IR parameters
#[derive(Debug)]
pub struct ParamMapping {
    inalloc_index: Option<usize>,
    sret_index: Option<usize>,
    llvm_arity: usize,
    params: Vec<Param>,
}

impl ParamMapping {
    pub fn new(_type_layout_cache: &TypeLayoutCache, abi_function: &ABIFunction) -> Self {
        let mut llvm_param_index = 0 as usize;
        let mut params = Vec::with_capacity(abi_function.parameter_types.len());
        let mut swap_this_with_sret = false;
        let mut inalloc_index = None;
        let mut sret_index = None;
        let return_info = &abi_function.return_type;

        if let ABITypeKind::Indirect(indirect) = &return_info.kind {
            swap_this_with_sret = indirect.sret_after_this;
            sret_index = if swap_this_with_sret {
                Some(1)
            } else {
                llvm_param_index += 1;
                Some(0)
            };
        }

        for abi_param in abi_function.parameter_types.iter() {
            let padding_index = if abi_param.padding_type().flatten().is_some() {
                let index = llvm_param_index;
                llvm_param_index += 1;
                Some(index)
            } else {
                None
            };

            let num_subparams = match &abi_param.kind {
                ABITypeKind::Direct(direct) => {
                    let struct_type = direct.coerce_to_type.filter(|llvm_type| {
                        return unsafe { LLVMGetTypeKind(*llvm_type) }
                            == LLVMTypeKind::LLVMStructTypeKind;
                    });

                    let num_subparams = if let Some(struct_type) = struct_type {
                        if direct.can_be_flattened {
                            usize::try_from(unsafe { LLVMCountStructElementTypes(struct_type) })
                                .unwrap()
                        } else {
                            1
                        }
                    } else {
                        1
                    };
                    num_subparams
                }
                ABITypeKind::Extend(_) => 1,
                ABITypeKind::Indirect(_) => 1,
                ABITypeKind::IndirectAliased(_) => 1,
                ABITypeKind::Ignore => 0,
                ABITypeKind::Expand(_) => todo!("param mapping for expand"),
                ABITypeKind::CoerceAndExpand(_) => todo!("param mapping for coerce and expand"),
                ABITypeKind::InAlloca(_) => 0,
            };

            let begin_index = llvm_param_index;
            llvm_param_index += num_subparams;

            // Compensate for already handling sret parameter
            if llvm_param_index == 1 && swap_this_with_sret {
                llvm_param_index += 1;
            }

            params.push(Param {
                padding_index,
                begin_index,
                num_subparams,
            });
        }

        if abi_function.inalloca_combined_struct.is_some() {
            inalloc_index = Some(llvm_param_index);
            llvm_param_index += 1;
        }

        Self {
            inalloc_index,
            sret_index,
            llvm_arity: llvm_param_index,
            params,
        }
    }

    pub fn inalloca(&self) -> Option<usize> {
        self.inalloc_index
    }

    pub fn sret_index(&self) -> Option<usize> {
        self.sret_index
    }

    pub fn llvm_args(&self, ir_param_index: usize) -> std::ops::Range<usize> {
        let param = &self.params[ir_param_index];
        param.begin_index..(param.begin_index + param.num_subparams)
    }

    pub fn llvm_arity(&self) -> usize {
        self.llvm_arity
    }

    pub fn arg_padding(&self, ir_param_index: usize) -> Option<usize> {
        self.params
            .get(ir_param_index)
            .and_then(|param| param.padding_index)
    }
}
