use crate::{
    data_units::{BitUnits, ByteUnits},
    ir,
    llvm_backend::{
        abi::{
            abi_function::{ABIFunction, ABIParam},
            abi_type::{ABIType, DirectOptions, ExtendOptions, IndirectOptions},
            cxx::Itanium,
            empty::{is_empty_record, IsEmptyRecordOptions},
            has_scalar_evaluation_kind,
        },
        backend_type::to_backend_type,
        ctx::ToBackendTypeCtx,
        error::BackendError,
    },
    target_info::{type_layout::TypeLayoutCache, TargetInfo},
};
use derive_more::IsVariant;
use llvm_sys::{
    core::{
        LLVMArrayType2, LLVMInt16Type, LLVMInt32Type, LLVMInt64Type, LLVMInt8Type, LLVMIntType,
    },
    prelude::LLVMTypeRef,
    LLVMCallConv,
};

#[derive(Clone, Debug)]
pub struct AARCH64<'a> {
    pub variant: Variant,
    pub target_info: &'a TargetInfo,
    pub type_layout_cache: &'a TypeLayoutCache<'a>,
    pub ir_module: &'a ir::Module,
    pub is_cxx_mode: bool,
}

#[derive(Copy, Clone, Debug, IsVariant)]
pub enum Variant {
    DarwinPCS,
    Win64,
    Aapcs,
    AapcsSoft,
}

#[allow(unused)]
impl AARCH64<'_> {
    pub fn compute_info(
        &self,
        ctx: &ToBackendTypeCtx<'_>,
        abi: Itanium<'_>,
        original_parameter_types: &[ir::Type],
        original_return_type: &ir::Type,
        is_variadic: bool,
    ) -> Result<ABIFunction, BackendError> {
        let return_type = ABIParam {
            ir_type: original_return_type.clone(),
            abi_type: abi
                .classify_return_type(original_return_type)
                .unwrap_or_else(|| self.classify_return_type(original_return_type, is_variadic)),
        };

        let mut parameter_types = Vec::new();

        for parameter in original_parameter_types.iter() {
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
        })
    }

    fn is_soft(&self) -> bool {
        self.variant.is_aapcs_soft()
    }

    fn classify_return_type(&self, return_type: &ir::Type, is_variadic: bool) -> ABIType {
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

        let type_layout = self.type_layout_cache.get(return_type);

        let size = type_layout.width;
        let alignment = type_layout.alignment;

        if size.is_zero()
            || is_empty_record(
                return_type,
                self.ir_module,
                IsEmptyRecordOptions {
                    allow_arrays: true,
                    ..Default::default()
                },
            )
        {
            return ABIType::new_ignore();
        }

        if !is_variadic
            && is_aarch64_homo_aggregate(
                self.variant,
                return_type,
                self.ir_module,
                None,
                self.type_layout_cache,
                self.target_info,
            )
            .is_some()
        {
            return ABIType::new_direct(DirectOptions::default());
        }

        // Small aggregates are returned directly via registers or stack
        if size <= ByteUnits::of(8) && self.target_info.is_little_endian() {
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

        get_natural_align_indirect(
            return_type,
            self.type_layout_cache,
            NaturalAlignIndirectOptions::default(),
        )
    }

    fn classify_argument_type(
        &self,
        ctx: &ToBackendTypeCtx<'_>,
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

        let size_bytes = self.type_layout_cache.get(ty).width;

        let is_empty_record = is_empty_record(
            ty,
            self.ir_module,
            IsEmptyRecordOptions {
                allow_arrays: true,
                ..Default::default()
            },
        );

        // NOTE: C++ records with non-trivial destructors/copy-constructors
        // would need to be passed as indirect, but we don't support those yet.

        // Empty records are ignored in Darwin for C, but not C++.
        if is_empty_record || size_bytes.is_zero() {
            if !self.is_cxx_mode || self.variant.is_darwin_pcs() {
                return Ok(ABIType::new_ignore());
            }

            if is_empty_record && size_bytes.is_zero() {
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
            if let Some(homo_aggregate) = is_aarch64_homo_aggregate(
                self.variant,
                ty,
                self.ir_module,
                None,
                self.type_layout_cache,
                self.target_info,
            ) {
                if !self.variant.is_aapcs() {
                    let base = unsafe { to_backend_type(ctx, homo_aggregate.base)? };
                    return Ok(ABIType::new_direct(DirectOptions {
                        coerce_to_type: Some(unsafe {
                            LLVMArrayType2(base, homo_aggregate.num_members)
                        }),
                        ..Default::default()
                    }));
                }
            }
        }

        if size_bytes <= ByteUnits::of(16) {
            let alignment_bytes = if self.variant.is_aapcs() {
                let unadjusted_alignment = self.type_layout_cache.get(ty).unadjusted_alignment;

                if unadjusted_alignment < ByteUnits::of(16) {
                    ByteUnits::of(8)
                } else {
                    ByteUnits::of(16)
                }
            } else {
                let pointer_width = ByteUnits::of(8);
                self.type_layout_cache.get(ty).alignment.max(pointer_width)
            };

            let size_bytes = size_bytes.align_to(alignment_bytes);

            let base_type = unsafe {
                LLVMIntType(
                    BitUnits::from(alignment_bytes)
                        .bits()
                        .try_into()
                        .expect("small enough for int type"),
                )
            };

            let coerce_to_type = if size_bytes == alignment_bytes {
                base_type
            } else {
                unsafe { LLVMArrayType2(base_type, size_bytes / alignment_bytes) }
            };

            return Ok(ABIType::new_direct(DirectOptions {
                coerce_to_type: Some(coerce_to_type),
                ..Default::default()
            }));
        }

        Ok(get_natural_align_indirect(
            ty,
            self.type_layout_cache,
            NaturalAlignIndirectOptions {
                byval: false,
                ..Default::default()
            },
        ))
    }

    fn coerce_illegal_vector(&self, ty: &ir::Type) -> ABIType {
        todo!("coerce_illegal_vector")
    }

    fn is_homo_aggregate_base_type(&self, ty: &ir::Type) -> bool {
        todo!("is_homo_aggregate_base_type")
    }

    fn is_homo_aggregate_small_enough(&self, ty: &ir::Type, members: u64) -> bool {
        todo!("is_homo_aggregate_small_enough")
    }

    fn is_zero_length_bitfield_allowed_in_homo_aggregrate(&self) -> bool {
        todo!("is_zero_length_bitfield_allowed_in_homo_aggregrate")
    }

    fn is_illegal_vector_type(&self, ty: &ir::Type) -> bool {
        if ty.is_vector() {
            todo!("is_illegal_vector_type")
        }
        false
    }
}

fn is_aggregate_type_for_abi(ty: &ir::Type) -> bool {
    !has_scalar_evaluation_kind(ty) || ty.is_function_pointer()
}

fn is_promotable_integer_type_for_abi(ty: &ir::Type) -> bool {
    // NOTE: Arbitrarily sized integers and `char32` should be, but we don't support those yet

    match ty {
        ir::Type::Boolean | ir::Type::S8 | ir::Type::S16 | ir::Type::U8 | ir::Type::U16 => true,
        ir::Type::S32
        | ir::Type::S64
        | ir::Type::U32
        | ir::Type::U64
        | ir::Type::F32
        | ir::Type::F64
        | ir::Type::Pointer(_)
        | ir::Type::Void
        | ir::Type::Union(_)
        | ir::Type::Structure(_)
        | ir::Type::AnonymousComposite(_)
        | ir::Type::FunctionPointer
        | ir::Type::FixedArray(_)
        | ir::Type::Vector(_)
        | ir::Type::Complex(_)
        | ir::Type::Atomic(_)
        | ir::Type::IncompleteArray(_) => false,
    }
}

#[derive(Copy, Clone, Debug)]
pub struct HomoAggregate<'a> {
    base: &'a ir::Type,
    num_members: u64,
}

fn is_aarch64_homo_aggregate<'a>(
    variant: Variant,
    ty: &'a ir::Type,
    ir_module: &'a ir::Module,
    existing_base: Option<&'a ir::Type>,
    type_layout_cache: &TypeLayoutCache,
    target_info: &TargetInfo,
) -> Option<HomoAggregate<'a>> {
    let homo_aggregate: Option<HomoAggregate<'a>> = if let ir::Type::FixedArray(fixed_array) = ty {
        if fixed_array.length == 0 {
            return None;
        }

        is_aarch64_homo_aggregate(
            variant,
            &fixed_array.inner,
            ir_module,
            existing_base,
            type_layout_cache,
            target_info,
        )
        .map(|homo_aggregate| HomoAggregate {
            base: homo_aggregate.base,
            num_members: homo_aggregate.num_members * fixed_array.length,
        })
    } else if let ir::Type::Structure(structure_ref) = ty {
        let structure = ir_module
            .structures
            .get(structure_ref)
            .expect("referenced structure to exist");
        is_aarch64_homo_aggregate_record(
            variant,
            ty,
            ir_module,
            &structure.fields[..],
            existing_base,
            type_layout_cache,
            target_info,
        )
    } else if let ir::Type::AnonymousComposite(anonymous_composite) = ty {
        is_aarch64_homo_aggregate_record(
            variant,
            ty,
            ir_module,
            &anonymous_composite.fields[..],
            existing_base,
            type_layout_cache,
            target_info,
        )
    } else {
        let (ty, num_members) = if let ir::Type::Complex(complex) = ty {
            (&complex.element_type, 2)
        } else {
            (ty, 1)
        };

        if !is_aarch64_homo_aggregate_base_type(ty, type_layout_cache, variant) {
            return None;
        }

        let base = &ty;

        if existing_base.is_none() {
            if let ir::Type::Vector(vector) = base {
                let element_type = &vector.element_type;
                let num_elements = vector.num_elements;

                assert_eq!(
                    num_elements,
                    type_layout_cache.get(base).width / type_layout_cache.get(element_type).width,
                );
            }

            if base.is_vector() != ty.is_vector()
                || type_layout_cache.get(ty).width != type_layout_cache.get(base).width
            {
                return None;
            }
        }

        Some(HomoAggregate { base, num_members })
    };

    homo_aggregate.filter(|homo_aggregate| {
        homo_aggregate.num_members > 0 && is_aarch64_homo_aggregate_small_enough(homo_aggregate)
    })
}

fn is_aarch64_homo_aggregate_record<'a>(
    variant: Variant,
    ty: &'a ir::Type,
    ir_module: &'a ir::Module,
    fields: &'a [ir::Field],
    existing_base: Option<&'a ir::Type>,
    type_layout_cache: &TypeLayoutCache,
    target_info: &TargetInfo,
) -> Option<HomoAggregate<'a>> {
    /*
    // NOTE: We don't support flexible array members yet
    if has_flexible_array_member() {
        return None;
    }
    */

    let mut base = existing_base;
    let mut num_combined_members = 0;

    // NOTE: We would need to check the bases as well if this was a C++ record type,
    // but we don't support those yet.

    for field in fields.iter() {
        let mut field = &field.ir_type;

        // Ignore non-zero arrays of empty records
        while let ir::Type::FixedArray(fixed_array) = field {
            if fixed_array.length == 0 {
                return None;
            }

            field = &fixed_array.inner;
        }

        if is_empty_record(
            field,
            ir_module,
            IsEmptyRecordOptions {
                allow_arrays: true,
                ..Default::default()
            },
        ) {
            continue;
        }

        /*
        // NOTE: We don't support bit fields yet, otherwise we'd need something like
        if is_zero_length_bitfield_allowed_in_homo_aggregrate() && is_zero_length_bitfield(field) {
            continue;
        }
        */

        if let Some(inner) = is_aarch64_homo_aggregate(
            variant,
            field,
            ir_module,
            base,
            type_layout_cache,
            target_info,
        ) {
            // NOTE: We don't support union types yet
            let is_union = false;
            base = Some(inner.base);

            num_combined_members = if is_union {
                num_combined_members.max(inner.num_members)
            } else {
                num_combined_members + inner.num_members
            };
        } else {
            return None;
        }
    }

    let base = base?;

    if type_layout_cache.get(base).width * num_combined_members != type_layout_cache.get(ty).width {
        return None;
    }

    Some(HomoAggregate {
        base,
        num_members: num_combined_members,
    })
}

#[derive(Copy, Clone, Debug)]
pub struct NaturalAlignIndirectOptions {
    pub byval: bool,
    pub realign: bool,
    pub padding: Option<LLVMTypeRef>,
}

impl Default for NaturalAlignIndirectOptions {
    fn default() -> Self {
        Self {
            byval: true,
            realign: false,
            padding: None,
        }
    }
}

fn get_natural_align_indirect(
    ir_type: &ir::Type,
    type_layout_cache: &TypeLayoutCache,
    options: NaturalAlignIndirectOptions,
) -> ABIType {
    let alignment = type_layout_cache.get(ir_type).alignment;

    ABIType::new_indirect(
        alignment,
        Some(options.byval),
        Some(options.realign),
        options.padding,
        IndirectOptions::default(),
    )
}

fn is_aarch64_homo_aggregate_base_type(
    ty: &ir::Type,
    type_layout_cache: &TypeLayoutCache,
    variant: Variant,
) -> bool {
    if variant.is_aapcs_soft() {
        return false;
    }

    match ty {
        ir::Type::F32 | ir::Type::F64 => true,
        ir::Type::Vector(_) => {
            let size = type_layout_cache.get(ty).width;
            size == ByteUnits::of(8) || size == ByteUnits::of(16)
        }
        _ => false,
    }
}

fn is_aarch64_homo_aggregate_small_enough(homo_aggregate: &HomoAggregate<'_>) -> bool {
    homo_aggregate.num_members <= 4
}

fn use_first_field_if_transparent_union(ty: &ir::Type) -> &ir::Type {
    // NOTE: We don't support transparent unions yet
    ty
}
