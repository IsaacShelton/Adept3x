use super::use_first_field_if_transparent_union;
use crate::{
    BackendError,
    abi::{
        abi_function::{ABIFunction, ABIParam},
        abi_type::{ABIType, DirectOptions, ExtendOptions},
        cxx::Itanium,
        empty::{IsEmptyRecordOptions, is_empty_record},
        homo_aggregate::{HomoAggregate, HomoDecider, is_homo_aggregate},
        is_aggregate_type_for_abi, is_promotable_integer_type_for_abi,
    },
    backend_type::to_backend_type,
    ctx::BackendCtx,
    llvm_type_ref_ext::LLVMTypeRefExt,
};
use data_units::ByteUnits;
use derive_more::IsVariant;
use llvm_sys::{
    LLVMCallConv,
    core::{LLVMArrayType2, LLVMInt8Type, LLVMInt16Type, LLVMInt32Type, LLVMInt64Type},
    prelude::LLVMTypeRef,
};
use target_layout::TypeLayoutCache;

#[derive(Clone, Debug)]
pub struct Aarch64 {
    pub variant: Aarch64Variant,
    pub is_cxx_mode: bool,
}

#[derive(Copy, Clone, Debug, IsVariant)]
pub enum Aarch64Variant {
    DarwinPCS,
    Win64,
    Aapcs,
    AapcsSoft,
}

impl Aarch64 {
    pub fn compute_info<'a>(
        &self,
        ctx: &BackendCtx,
        abi: Itanium,
        original_parameter_types: impl Iterator<Item = &'a ir::Type>,
        original_return_type: &ir::Type,
        is_variadic: bool,
    ) -> Result<ABIFunction, BackendError> {
        let return_type = ABIParam {
            ir_type: original_return_type.clone(),
            abi_type: abi
                .classify_return_type(original_return_type)
                .unwrap_or_else(|| {
                    self.classify_return_type(ctx, original_return_type, is_variadic)
                }),
        };

        let mut parameter_types = Vec::new();

        for parameter in original_parameter_types {
            parameter_types.push(ABIParam {
                abi_type: self.classify_argument_type(
                    ctx,
                    parameter,
                    is_variadic,
                    LLVMCallConv::LLVMCCallConv,
                )?,
                ir_type: parameter.clone(),
            });
        }

        Ok(ABIFunction {
            parameter_types,
            return_type,
            inalloca_combined_struct: None,
            head_max_vector_width: ByteUnits::of(0),
        })
    }

    fn classify_return_type(
        &self,
        ctx: &BackendCtx,
        return_type: &ir::Type,
        is_variadic: bool,
    ) -> ABIType {
        if return_type.is_void() {
            return ABIType::new_ignore();
        }

        if return_type.is_vector() {
            todo!("classify_return_type for ir::Type::Vector");
        }

        if !is_aggregate_type_for_abi(return_type) {
            return if is_promotable_integer_type_for_abi(return_type)
                && self.variant.is_darwin_pcs()
            {
                ABIType::new_extend(return_type, None, ExtendOptions::default())
            } else {
                ABIType::new_direct(DirectOptions::default())
            };
        }

        let type_layout = ctx.type_layout_cache.get(return_type);

        let size = type_layout.width;
        let alignment = type_layout.alignment;

        if size.is_zero()
            || is_empty_record(
                return_type,
                ctx.ir_module,
                IsEmptyRecordOptions {
                    allow_arrays: true,
                    ..Default::default()
                },
            )
        {
            return ABIType::new_ignore();
        }

        if !is_variadic
            && self
                .is_homo_aggregate(return_type, ctx.ir_module, None, &ctx.type_layout_cache)
                .is_some()
        {
            return ABIType::new_direct(DirectOptions::default());
        }

        // Small aggregates are returned directly via registers or stack
        if size <= ByteUnits::of(8) && ctx.ir_module.target.is_little_endian() {
            let ty = match size.bytes() {
                1 => unsafe { LLVMInt8Type() },
                2 => unsafe { LLVMInt16Type() },
                3..=4 => unsafe { LLVMInt32Type() },
                5..=8 => unsafe { LLVMInt64Type() },
                _ => panic!("expected aggregate to be register sized"),
            };

            return ABIType::new_direct(DirectOptions {
                coerce_to_type: Some(ty),
                ..Default::default()
            });
        }

        let size = size.align_to(ByteUnits::of(8));

        if alignment < ByteUnits::of(16) && size == ByteUnits::of(16) {
            let base_type = unsafe { LLVMInt64Type() };
            let num_elements = size / ByteUnits::of(8);
            let array_type = unsafe { LLVMArrayType2(base_type, num_elements) };

            return ABIType::new_direct(DirectOptions {
                coerce_to_type: Some(array_type),
                ..Default::default()
            });
        }

        ABIType::new_indirect_natural_align(&ctx.type_layout_cache, return_type, None, None, None)
    }

    fn classify_argument_type(
        &self,
        ctx: &BackendCtx,
        ty: &ir::Type,
        is_variadic: bool,
        calling_convention: LLVMCallConv,
    ) -> Result<ABIType, BackendError> {
        let ty = use_first_field_if_transparent_union(ty);

        if self.is_illegal_vector_type(ty) {
            return Ok(self.coerce_illegal_vector(ty));
        }

        if !is_aggregate_type_for_abi(ty) {
            // NOTE: We don't support arbitrarily sized integers,
            // but if we did we would have to compensate for them here.

            return if is_promotable_integer_type_for_abi(ty) && self.variant.is_darwin_pcs() {
                Ok(ABIType::new_extend(ty, None, ExtendOptions::default()))
            } else {
                Ok(ABIType::new_direct(DirectOptions::default()))
            };
        }

        let size = ctx.type_layout_cache.get(ty).width;

        let is_empty_record = is_empty_record(
            ty,
            ctx.ir_module,
            IsEmptyRecordOptions {
                allow_arrays: true,
                ..Default::default()
            },
        );

        // NOTE: C++ records with non-trivial destructors/copy-constructors
        // would need to be passed as indirect, but we don't support those yet.

        // Empty records are ignored in Darwin for C, but not C++.
        if is_empty_record || size.is_zero() {
            if !self.is_cxx_mode || self.variant.is_darwin_pcs() {
                return Ok(ABIType::new_ignore());
            }

            if size.is_zero() {
                return Ok(ABIType::new_ignore());
            }

            return Ok(ABIType::new_direct(DirectOptions {
                coerce_to_type: Some(unsafe { LLVMInt8Type() }),
                ..Default::default()
            }));
        }

        let is_win64 =
            self.variant.is_win_64() || calling_convention == LLVMCallConv::LLVMWin64CallConv;

        let is_win64_variadic = is_win64 && is_variadic;

        // For variadic functions on Windows, all composites are treated the same,
        // so no special treatment for homogenous aggregates.
        if !is_win64_variadic {
            if let Some(homo_aggregate) =
                self.is_homo_aggregate(ty, ctx.ir_module, None, &ctx.type_layout_cache)
            {
                if !self.variant.is_aapcs() {
                    let base =
                        unsafe { to_backend_type(ctx.for_making_type(), homo_aggregate.base)? };
                    return Ok(ABIType::new_direct(DirectOptions {
                        coerce_to_type: Some(unsafe {
                            LLVMArrayType2(base, homo_aggregate.num_members.into())
                        }),
                        ..Default::default()
                    }));
                }
            }
        }

        if size <= ByteUnits::of(16) {
            let alignment = if self.variant.is_aapcs() {
                let unadjusted_alignment = ctx.type_layout_cache.get(ty).unadjusted_alignment;

                if unadjusted_alignment < ByteUnits::of(16) {
                    ByteUnits::of(8)
                } else {
                    ByteUnits::of(16)
                }
            } else {
                let pointer_width = ByteUnits::of(8);
                ctx.type_layout_cache.get(ty).alignment.max(pointer_width)
            };

            let size = size.align_to(alignment);
            let base_type = LLVMTypeRef::new_int(alignment);

            let coerce_to_type = if size == alignment {
                base_type
            } else {
                let length = size / alignment;
                LLVMTypeRef::new_array(base_type, length)
            };

            return Ok(ABIType::new_direct(DirectOptions {
                coerce_to_type: Some(coerce_to_type),
                ..Default::default()
            }));
        }

        Ok(ABIType::new_indirect_natural_align(
            &ctx.type_layout_cache,
            ty,
            Some(false),
            None,
            None,
        ))
    }

    fn is_homo_aggregate<'a>(
        &self,
        ir_type: &'a ir::Type,
        ir_module: &'a ir::Module,
        existing_base: Option<&'a ir::Type>,
        type_layout_cache: &TypeLayoutCache,
    ) -> Option<HomoAggregate<'a>> {
        is_homo_aggregate(
            &Aarch64HomoDecider {
                variant: self.variant,
            },
            ir_type,
            ir_module,
            existing_base,
            type_layout_cache,
        )
    }

    fn coerce_illegal_vector(&self, _ty: &ir::Type) -> ABIType {
        todo!("coerce_illegal_vector")
    }

    fn is_illegal_vector_type(&self, ty: &ir::Type) -> bool {
        if ty.is_vector() {
            todo!("is_illegal_vector_type")
        }
        false
    }
}

struct Aarch64HomoDecider {
    variant: Aarch64Variant,
}

impl HomoDecider for Aarch64HomoDecider {
    fn is_base_type(&self, ir_type: &ir::Type, type_layout_cache: &TypeLayoutCache) -> bool {
        if self.variant.is_aapcs_soft() {
            return false;
        }

        match ir_type {
            ir::Type::F32 | ir::Type::F64 => true,
            ir::Type::Vector(_) => matches!(type_layout_cache.get(ir_type).width.bytes(), 8 | 16),
            _ => false,
        }
    }

    fn is_small_enough(&self, homo_aggregate: &HomoAggregate<'_>) -> bool {
        homo_aggregate.num_members <= 4
    }
}
