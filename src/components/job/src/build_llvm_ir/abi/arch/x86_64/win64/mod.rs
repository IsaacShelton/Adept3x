use super::AvxLevel;
use crate::{
    build_llvm_ir::{
        abi::{
            abi_function::{ABIFunction, ABIParam},
            abi_type::{ABIType, DirectOptions, ExtendOptions, IndirectOptions},
            cxx::Itanium,
            homo_aggregate::{HomoAggregate, HomoDecider, is_homo_aggregate},
        },
        ctx::BackendCtx,
    },
    ir,
    target_layout::TypeLayoutCache,
};
use data_units::ByteUnits;
use diagnostics::ErrorDiagnostic;
use itertools::Itertools;
use llvm_sys::LLVMCallConv;

#[derive(Clone, Debug)]
pub struct Win64 {
    pub avx_level: AvxLevel,
    pub is_mingw: bool,
}

impl Win64 {
    pub fn new(avx_level: AvxLevel) -> Self {
        Self {
            avx_level,
            is_mingw: true,
        }
    }

    pub fn compute_info<'env>(
        &self,
        ctx: &BackendCtx<'_, 'env>,
        abi: &Itanium<'_, 'env>,
        original_parameter_types: impl Iterator<Item = &'env ir::Type<'env>>,
        original_return_type: &'env ir::Type<'env>,
        calling_convention: LLVMCallConv,
    ) -> Result<ABIFunction<'env>, ErrorDiagnostic> {
        let is_vector_call = calling_convention == LLVMCallConv::LLVMX86VectorCallCallConv;
        let is_reg_call = calling_convention == LLVMCallConv::LLVMX86RegCallCallConv;
        let free_return_sses = if is_vector_call { 4 } else { 16 };

        let abi_return_type = abi
            .classify_return_type(original_return_type)
            .unwrap_or_else(|| {
                self.classify(
                    ctx,
                    abi,
                    original_return_type,
                    free_return_sses,
                    true,
                    is_vector_call,
                    is_reg_call,
                )
                .abi_type
            });

        let return_type = ABIParam {
            ir_type: original_return_type,
            abi_type: abi_return_type,
        };

        let mut free_sses = if is_vector_call { 6 } else { 16 };

        let parameter_types = original_parameter_types
            .enumerate()
            .map(|(i, parameter)| {
                let parameter_free_sses = if is_vector_call && i >= 6 {
                    0
                } else {
                    free_sses
                };

                let requirement = self.classify(
                    ctx,
                    abi,
                    parameter,
                    parameter_free_sses,
                    false,
                    is_vector_call,
                    is_reg_call,
                );

                free_sses -= requirement.sses;
                ABIParam {
                    abi_type: requirement.abi_type,
                    ir_type: parameter,
                }
            })
            .collect_vec();

        let parameter_types = if is_vector_call {
            parameter_types
                .into_iter()
                .map(|parameter| {
                    let requirement = self.reclassify_homo_vector_aggregate_arg_for_vector_call(
                        ctx,
                        parameter.abi_type,
                        parameter.ir_type,
                        free_sses,
                    );

                    free_sses -= requirement.sses;
                    ABIParam {
                        abi_type: requirement.abi_type,
                        ir_type: parameter.ir_type,
                    }
                })
                .collect_vec()
        } else {
            parameter_types
        };

        Ok(ABIFunction {
            parameter_types,
            return_type,
            inalloca_combined_struct: None,
            head_max_vector_width: ByteUnits::of(0),
        })
    }

    fn classify<'env>(
        &self,
        ctx: &BackendCtx<'_, 'env>,
        abi: &Itanium<'_, 'env>,
        ir_type: &'env ir::Type<'env>,
        free_sses: u32,
        is_return_type: bool,
        is_vector_call: bool,
        is_reg_call: bool,
    ) -> Requirement {
        if ir_type.is_void() {
            return Requirement::new(ABIType::new_ignore(), 0);
        }

        let layout = ctx.type_layout_cache.get(ir_type);

        if ir_type.is_product_type() || ir_type.is_union() {
            if !is_return_type {
                let record_arg_abi = abi.get_record_arg_abi(ir_type);
                if !record_arg_abi.is_default() {
                    return Requirement::new(
                        ABIType::new_indirect_natural_align(
                            &ctx.type_layout_cache,
                            ir_type,
                            Some(record_arg_abi.is_direct_in_memory()),
                            None,
                            None,
                        ),
                        0,
                    );
                }

                if ir_type.has_flexible_array_member() {
                    return Requirement::new(
                        ABIType::new_indirect_natural_align(
                            &ctx.type_layout_cache,
                            ir_type,
                            Some(false),
                            None,
                            None,
                        ),
                        0,
                    );
                }
            }
        }

        if is_vector_call || is_reg_call {
            if let Some(homo_aggregate) =
                self.is_homo_aggregate(ir_type, ctx.ir_module, None, &ctx.type_layout_cache)
            {
                if is_reg_call {
                    if free_sses >= homo_aggregate.num_members {
                        if is_return_type || ir_type.is_builtin_data() || ir_type.is_vector() {
                            return Requirement::new(
                                ABIType::new_direct(DirectOptions::default()),
                                homo_aggregate.num_members,
                            );
                        }

                        return Requirement::new(ABIType::new_expand(), homo_aggregate.num_members);
                    }

                    return Requirement::new(
                        ABIType::new_indirect(
                            layout.alignment,
                            Some(false),
                            None,
                            None,
                            IndirectOptions::default(),
                        ),
                        0,
                    );
                } else if is_vector_call {
                    if free_sses >= homo_aggregate.num_members
                        && (is_return_type || ir_type.is_builtin_data() || ir_type.is_vector())
                    {
                        return Requirement::new(
                            ABIType::new_direct(DirectOptions::default()),
                            homo_aggregate.num_members,
                        );
                    }
                } else if is_return_type {
                    return Requirement::new(ABIType::new_expand(), 0);
                } else if !ir_type.is_builtin_data() && !ir_type.is_vector() {
                    return Requirement::new(
                        ABIType::new_indirect(
                            layout.alignment,
                            Some(false),
                            None,
                            None,
                            IndirectOptions::default(),
                        ),
                        0,
                    );
                }
            }
        }

        // NOTE: We don't support member pointer types here

        // NOTE: We don't support long doubles, or i128/u128 128-bit integers here
        if ir_type.is_bool() {
            return Requirement::new(
                ABIType::new_extend(ir_type, None, ExtendOptions::default()),
                0,
            );
        }

        // NOTE: We don't support arbitrarily sized integers here
        Requirement::new(ABIType::new_direct(DirectOptions::default()), 0)
    }

    fn reclassify_homo_vector_aggregate_arg_for_vector_call<'env>(
        &self,
        ctx: &BackendCtx<'_, 'env>,
        current: ABIType,
        ir_type: &'env ir::Type<'env>,
        free_sses: u32,
    ) -> Requirement {
        if !ir_type.is_builtin_data() && !ir_type.is_vector() {
            if let Some(homo_aggregate) =
                self.is_homo_aggregate(ir_type, ctx.ir_module, None, &ctx.type_layout_cache)
            {
                if free_sses >= homo_aggregate.num_members {
                    let homo_vector_aggregate = ABIType::new_direct(DirectOptions {
                        can_be_flattened: false,
                        in_register: true,
                        ..Default::default()
                    });

                    return Requirement::new(homo_vector_aggregate, homo_aggregate.num_members);
                }
            }
        }

        Requirement::new(current, 0)
    }

    pub fn is_homo_aggregate<'env>(
        &self,
        ty: &'env ir::Type<'env>,
        ir_module: &'env ir::Ir<'env>,
        existing_base: Option<&'env ir::Type>,
        type_layout_cache: &TypeLayoutCache<'env>,
    ) -> Option<HomoAggregate<'env>> {
        is_homo_aggregate(
            &Win64HomoDecider,
            ty,
            ir_module,
            existing_base,
            type_layout_cache,
        )
    }
}

struct Win64HomoDecider;

impl<'env> HomoDecider<'env> for Win64HomoDecider {
    fn is_base_type(
        &self,
        ir_type: &'env ir::Type<'env>,
        type_layout_cache: &TypeLayoutCache<'env>,
    ) -> bool {
        // NOTE: We don't support long doubles here
        match ir_type {
            ir::Type::F(..) => true,
            ir::Type::Vector(_) => {
                matches!(type_layout_cache.get(ir_type).width.bytes(), 16 | 32 | 64)
            }
            _ => false,
        }
    }

    fn is_small_enough(&self, homo_aggregate: &HomoAggregate<'_>) -> bool {
        homo_aggregate.num_members <= 4
    }
}

#[derive(Clone, Debug)]
struct Requirement {
    pub abi_type: ABIType,
    pub sses: u32,
}

impl Requirement {
    pub fn new(abi_type: ABIType, sses: u32) -> Self {
        Self { abi_type, sses }
    }
}
