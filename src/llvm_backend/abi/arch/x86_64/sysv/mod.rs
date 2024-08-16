use super::{avx_level::AvxLevel, reg_count::RegCount};
use crate::{
    data_units::{BitUnits, ByteUnits},
    ir,
    llvm_backend::{
        abi::{
            abi_function::{ABIFunction, ABIParam},
            abi_type::{ABIType, DirectOptions, ExtendOptions, IndirectOptions},
            arch::use_first_field_if_transparent_union,
            cxx::Itanium,
            empty::{is_empty_field, IsEmptyRecordOptions},
            is_aggregate_type_for_abi, is_promotable_integer_type_for_abi,
        },
        backend_type::to_backend_type,
        ctx::BackendCtx,
        error::BackendError,
        llvm_type_ref_ext::LLVMTypeRefExt,
    },
    target_info::{
        record_layout::{itanium::ItaniumRecordLayoutBuilder, record_info::RecordInfo},
        type_layout::{TypeLayout, TypeLayoutCache},
    },
};
use derive_more::IsVariant;
use llvm_sys::{
    core::{LLVMDoubleType, LLVMInt64Type, LLVMStructType, LLVMVectorType},
    prelude::LLVMTypeRef,
    target::{LLVMABIAlignmentOfType, LLVMElementAtOffset, LLVMOffsetOfElement},
    LLVMCallConv,
};
use reg_class::RegClass;
use reg_class_pair::RegClassPair;
use std::ptr::null_mut;

mod reg_class;
mod reg_class_pair;

#[derive(Clone, Debug, IsVariant)]
pub enum SysVOs {
    Darwin,
    Linux,
    Bsd,
}

#[derive(Clone, Debug)]
pub struct SysV {
    pub os: SysVOs,
    pub avx_level: AvxLevel,
}

impl SysV {
    fn honors_revision_0_98(&self) -> bool {
        !self.os.is_darwin()
    }

    fn post_merge(&self, aggregate_size: ByteUnits, mut pair: RegClassPair) -> RegClassPair {
        use RegClass::*;

        if pair.high == Memory {
            pair.low = Memory;
        } else if pair.high == X87Up && pair.low != X87 && self.honors_revision_0_98() {
            pair.low = Memory;
        } else if pair.high == SseUp && pair.low != Sse {
            pair.high = Sse;
        } else if aggregate_size > ByteUnits::of(16) && pair.low != Sse {
            pair.low = Memory;
        }

        pair
    }

    fn classify_return_type(
        &self,
        ctx: &BackendCtx,
        abi: &Itanium,
        ir_type: &ir::Type,
    ) -> Result<ABIType, BackendError> {
        let pair = self.classify(ctx, abi, ir_type, ByteUnits::of(0), true, false);

        assert!(!pair.high.is_memory() || pair.low.is_memory());
        assert!(!pair.high.is_sse_up() || pair.low.is_sse());

        let llvm_type = unsafe { to_backend_type(ctx.for_making_type(), ir_type)? };

        let mut result_type = match pair.low {
            RegClass::NoClass => {
                if pair.high.is_no_class() {
                    return Ok(ABIType::new_ignore());
                }

                let high_part = match pair.high {
                    RegClass::Sse | RegClass::X87Up => Self::get_sse_type_at_offset(
                        ctx,
                        llvm_type,
                        ByteUnits::of(8),
                        ir_type,
                        ByteUnits::of(8),
                    ),
                    RegClass::Integer => Self::get_integer_type_at_offset(
                        ctx,
                        llvm_type,
                        ByteUnits::of(8),
                        ir_type,
                        ByteUnits::of(8),
                    ),
                    _ => panic!("unknown missing low part"),
                };

                return Ok(ABIType::new_direct(DirectOptions {
                    coerce_to_type: Some(high_part),
                    offset: ByteUnits::of(8),
                    ..Default::default()
                }));
            }
            RegClass::Integer => {
                let result_type = Self::get_integer_type_at_offset(
                    ctx,
                    llvm_type,
                    ByteUnits::of(0),
                    ir_type,
                    ByteUnits::of(0),
                );

                if pair.high.is_no_class()
                    && result_type.is_integer()
                    && is_promotable_integer_type_for_abi(ir_type)
                {
                    return Ok(ABIType::new_extend(ir_type, None, ExtendOptions::default()));
                }

                result_type
            }
            RegClass::Sse => Self::get_sse_type_at_offset(
                ctx,
                llvm_type,
                ByteUnits::of(0),
                ir_type,
                ByteUnits::of(0),
            ),
            RegClass::X87 => todo!("x86 fp80 not supported yet in SysV::classify_return_type"),
            RegClass::X87Up | RegClass::SseUp => unreachable!(),
            RegClass::ComplexX87 => {
                todo!("complex x86 fp80 not supported yet in SysV::classify_return_type")
            }
            RegClass::Memory => {
                return Ok(Self::get_indirect_return_result(
                    &ctx.type_layout_cache,
                    ir_type,
                ));
            }
        };

        let high_part = match pair.high {
            RegClass::Integer => Some(Self::get_integer_type_at_offset(
                ctx,
                llvm_type,
                ByteUnits::of(8),
                ir_type,
                ByteUnits::of(8),
            )),
            RegClass::Sse => Some(Self::get_sse_type_at_offset(
                ctx,
                llvm_type,
                ByteUnits::of(8),
                ir_type,
                ByteUnits::of(8),
            )),

            RegClass::SseUp => {
                assert_eq!(pair.low, RegClass::Sse);
                result_type = self.get_byte_vector_type(ctx, ir_type)?;
                None
            }
            RegClass::X87Up => {
                if !pair.low.is_x_87() {
                    Some(Self::get_sse_type_at_offset(
                        ctx,
                        llvm_type,
                        ByteUnits::of(8),
                        ir_type,
                        ByteUnits::of(8),
                    ))
                } else {
                    None
                }
            }
            RegClass::NoClass | RegClass::ComplexX87 => None,
            RegClass::Memory | RegClass::X87 => {
                unreachable!("invalid high part in SysV::classify_return_type")
            }
        };

        let result_type = if let Some(high_part) = high_part {
            Self::make_byval_argument_pair(ctx, result_type, high_part)
        } else {
            result_type
        };

        Ok(ABIType::new_direct(DirectOptions {
            coerce_to_type: Some(result_type),
            ..Default::default()
        }))
    }

    pub fn compute_info<'a>(
        &self,
        ctx: &BackendCtx,
        abi: &Itanium,
        original_parameter_types: impl Iterator<Item = &'a ir::Type>,
        num_required: usize,
        original_return_type: &ir::Type,
        calling_convention: LLVMCallConv,
    ) -> Result<ABIFunction, BackendError> {
        let is_reg_call = calling_convention == LLVMCallConv::LLVMX86RegCallCallConv;

        let mut free = if is_reg_call {
            RegCount::ints(11) + RegCount::sses(6)
        } else {
            RegCount::ints(16) + RegCount::sses(8)
        };

        let Requirement {
            abi_type: abi_return_type,
            needed: return_needed,
            max_vector_width: return_max_vector_width,
        } = if let Some(abi_type) = abi.classify_return_type(original_return_type) {
            Requirement::new(abi_type, RegCount::zeros(), ByteUnits::of(0))
        } else {
            // NOTE: We don't support long doubles here

            if is_reg_call && original_return_type.is_product_type() {
                let requirement =
                    self.classify_reg_call_struct_type(ctx, abi, original_return_type)?;

                if free.can_spare(requirement.needed) {
                    free -= requirement.needed;
                    requirement
                } else {
                    Requirement::new(
                        Self::get_indirect_return_result(
                            &ctx.type_layout_cache,
                            original_return_type,
                        ),
                        RegCount::zeros(),
                        ByteUnits::of(0),
                    )
                }
            } else {
                let abi_type = self.classify_return_type(ctx, abi, original_return_type)?;
                Requirement::new(abi_type, RegCount::zeros(), ByteUnits::of(0))
            }
        };

        let return_type = ABIParam {
            ir_type: original_return_type.clone(),
            abi_type: abi_return_type,
        };

        let mut head_max_vector_width = ByteUnits::of(0);

        // Indirect return value passed via int register
        if return_type.abi_type.is_indirect() {
            free -= RegCount::ints(1);
        } else if return_needed.has_sses(1) && !return_max_vector_width.is_zero() {
            head_max_vector_width = return_max_vector_width;
        }

        // NOTE: We don't support chain calling
        let mut parameter_types = Vec::new();

        for (i, parameter) in original_parameter_types.enumerate() {
            let is_required = i < num_required;

            let requirement = if is_reg_call && parameter.is_product_type() {
                self.classify_reg_call_struct_type(ctx, abi, parameter)?
            } else {
                self.classify_argument_type(ctx, abi, parameter, free, is_required, is_reg_call)?
            };

            let abi_type = if free.can_spare(requirement.needed) {
                free -= requirement.needed;
                head_max_vector_width = head_max_vector_width.max(requirement.max_vector_width);
                requirement.abi_type
            } else {
                self.get_indirect_result(ctx, abi, parameter, free)
            };

            parameter_types.push(ABIParam {
                abi_type,
                ir_type: parameter.clone(),
            });
        }

        Ok(ABIFunction {
            parameter_types,
            return_type,
            inalloca_combined_struct: None,
            head_max_vector_width,
        })
    }

    fn classify_argument_type(
        &self,
        ctx: &BackendCtx,
        abi: &Itanium,
        ir_type: &ir::Type,
        free: RegCount,
        is_required: bool,
        is_reg_call: bool,
    ) -> Result<Requirement, BackendError> {
        let ir_type = use_first_field_if_transparent_union(ir_type);

        let pair = self.classify(
            ctx,
            abi,
            ir_type,
            ByteUnits::of(0),
            is_required,
            is_reg_call,
        );

        assert!(
            pair.high != RegClass::Memory || pair.low == RegClass::Memory,
            "Invalid memory SysV classification"
        );

        assert!(
            pair.high != RegClass::SseUp || pair.low == RegClass::Sse,
            "Invalid memory SseUp classification"
        );

        let mut needed = RegCount::zeros();
        let mut result_type = null_mut();

        match pair.low {
            RegClass::NoClass => {
                if pair.high.is_no_class() {
                    return Ok(Requirement::new(
                        ABIType::new_ignore(),
                        RegCount::zeros(),
                        ByteUnits::of(0),
                    ));
                }

                assert!(pair.high.is_sse() || pair.high.is_integer() || pair.high.is_x_87_up());
            }
            RegClass::Integer => {
                let llvm_type = unsafe { to_backend_type(ctx.for_making_type(), ir_type)? };

                needed += RegCount::ints(1);

                result_type = Self::get_integer_type_at_offset(
                    ctx,
                    llvm_type,
                    ByteUnits::of(0),
                    ir_type,
                    ByteUnits::of(0),
                );

                if pair.high.is_no_class()
                    && result_type.is_integer()
                    && is_promotable_integer_type_for_abi(ir_type)
                {
                    return Ok(Requirement::new(
                        ABIType::new_extend(ir_type, None, ExtendOptions::default()),
                        needed,
                        ByteUnits::of(0),
                    ));
                }
            }
            RegClass::Sse => {
                let llvm_type = unsafe { to_backend_type(ctx.for_making_type(), ir_type)? };

                result_type = Self::get_sse_type_at_offset(
                    ctx,
                    llvm_type,
                    ByteUnits::of(0),
                    ir_type,
                    ByteUnits::of(0),
                );

                needed += RegCount::sses(1);
            }
            RegClass::SseUp | RegClass::X87Up => {
                unreachable!("invalid sysv register classification for low part")
            }
            RegClass::X87 | RegClass::ComplexX87 | RegClass::Memory => {
                if abi.get_record_arg_abi(ir_type).is_indirect() {
                    needed += RegCount::ints(1);
                }

                return Ok(Requirement::new(
                    self.get_indirect_result(ctx, abi, ir_type, free),
                    needed,
                    ByteUnits::of(0),
                ));
            }
        }

        let mut high_part = null_mut();

        match pair.high {
            RegClass::NoClass => (),
            RegClass::Integer => {
                let llvm_type = unsafe { to_backend_type(ctx.for_making_type(), ir_type) }?;

                needed += RegCount::ints(1);

                high_part = Self::get_integer_type_at_offset(
                    ctx,
                    llvm_type,
                    ByteUnits::of(8),
                    ir_type,
                    ByteUnits::of(8),
                );

                if pair.low.is_no_class() {
                    return Ok(Requirement::new(
                        ABIType::new_direct(DirectOptions {
                            coerce_to_type: Some(high_part),
                            offset: ByteUnits::of(8),
                            ..Default::default()
                        }),
                        needed,
                        ByteUnits::of(0),
                    ));
                }
            }
            RegClass::Sse | RegClass::X87Up => {
                let llvm_type = unsafe { to_backend_type(ctx.for_making_type(), ir_type) }?;

                needed += RegCount::sses(1);

                high_part = Self::get_sse_type_at_offset(
                    ctx,
                    llvm_type,
                    ByteUnits::of(8),
                    ir_type,
                    ByteUnits::of(8),
                );

                if pair.low.is_no_class() {
                    return Ok(Requirement::new(
                        ABIType::new_direct(DirectOptions {
                            coerce_to_type: Some(high_part),
                            offset: ByteUnits::of(8),
                            ..Default::default()
                        }),
                        needed,
                        ByteUnits::of(0),
                    ));
                }
            }
            RegClass::SseUp => {
                assert!(pair.low.is_sse());
                result_type = self.get_byte_vector_type(ctx, ir_type)?;
            }
            RegClass::X87 | RegClass::ComplexX87 | RegClass::Memory => {
                unreachable!("invalid sysv register classification for high part")
            }
        }

        if !high_part.is_null() {
            result_type = Self::make_byval_argument_pair(ctx, result_type, high_part);
        }

        Ok(Requirement::new(
            ABIType::new_direct(DirectOptions {
                coerce_to_type: Some(result_type),
                ..Default::default()
            }),
            needed,
            ByteUnits::of(0),
        ))
    }

    fn classify(
        &self,
        ctx: &BackendCtx,
        abi: &Itanium,
        ty: &ir::Type,
        offset_base: ByteUnits,
        is_required: bool,
        is_reg_call: bool,
    ) -> RegClassPair {
        let mut pair = RegClassPair::default();

        let current = if offset_base < ByteUnits::of(8) {
            &mut pair.low
        } else {
            &mut pair.high
        };

        *current = RegClass::Memory;

        // NOTE: We don't yet support long doubles, complex types, vector types,
        // or C++ member pointers

        match ty {
            ir::Type::Pointer(_)
            | ir::Type::FunctionPointer
            | ir::Type::Boolean
            | ir::Type::S8
            | ir::Type::S16
            | ir::Type::S32
            | ir::Type::S64
            | ir::Type::U8
            | ir::Type::U16
            | ir::Type::U32
            | ir::Type::U64 => *current = RegClass::Integer,
            ir::Type::F32 | ir::Type::F64 => *current = RegClass::Sse,
            ir::Type::Void => *current = RegClass::NoClass,
            ir::Type::Union(_) | ir::Type::Structure(_) | ir::Type::AnonymousComposite(_) => {
                pair = self.classify_record(ctx, abi, ty, offset_base, is_required, pair)
            }
            ir::Type::FixedArray(fixed_array) => {
                let size = ctx.type_layout_cache.get(ty).width;

                if !is_reg_call && size > ByteUnits::of(64) {
                    return pair;
                }

                let TypeLayout {
                    width: element_size,
                    alignment: element_alignment,
                    ..
                } = ctx.type_layout_cache.get(&fixed_array.inner);

                if !(offset_base % element_alignment).is_zero() {
                    return pair;
                }

                *current = RegClass::NoClass;

                if size > ByteUnits::of(16)
                    && (size != element_size || size > self.avx_level.native_vector_size())
                {
                    return pair;
                }

                for i in 0..fixed_array.length {
                    let offset = offset_base + element_size * i;
                    let field = self.classify(ctx, abi, ty, offset, is_required, is_reg_call);

                    pair.merge_with(field);

                    if pair.low == RegClass::Memory || pair.high == RegClass::Memory {
                        break;
                    }
                }

                self.post_merge(size, pair);

                assert!(pair.high != RegClass::SseUp || pair.low == RegClass::Sse);
            }
            ir::Type::Vector(_) => todo!("vector types with x86_64 sysv abi are not supported yet"),
            ir::Type::Complex(_) => {
                todo!("complex types with x86_64 sysv abi are not supported yet")
            }
            ir::Type::Atomic(inner) => {
                return self.classify(ctx, abi, inner, offset_base, is_required, is_reg_call);
            }
            ir::Type::IncompleteArray(_) => {
                todo!("incomplete array types with x86_64 sysv abi are not supported yet")
            }
        }

        pair
    }

    fn classify_record(
        &self,
        ctx: &BackendCtx,
        abi: &Itanium,
        ty: &ir::Type,
        offset_base: ByteUnits,
        is_required: bool,
        mut pair: RegClassPair,
    ) -> RegClassPair {
        let size = ctx.type_layout_cache.get(ty).width;

        if size > ByteUnits::of(64) {
            return pair;
        }

        let current = if offset_base < ByteUnits::of(8) {
            &mut pair.low
        } else {
            &mut pair.high
        };

        let record_arg_abi = abi.get_record_arg_abi(ty);

        if !record_arg_abi.is_default() {
            return pair;
        }

        if ty.has_flexible_array_member() {
            return pair;
        }

        let info = RecordInfo::try_from_type(ty, ctx.ir_module)
            .expect("invalid record type for SysV ABI classify_record");

        let layout = ItaniumRecordLayoutBuilder::generate(
            &ctx.type_layout_cache,
            ctx.type_layout_cache.diagnostics,
            &info,
            None,
        );

        *current = RegClass::NoClass;

        // NOTE: We don't support C++ inheritance
        let is_union = ty.is_union();

        assert_eq!(info.fields.len(), layout.field_offsets.len());

        for (field, offset) in info.fields.iter().zip(layout.field_offsets.iter()) {
            if field.is_bitfield() && field.is_unnamed() {
                continue;
            }

            if size > ByteUnits::of(16) {
                let field_size = ctx.type_layout_cache.get(&field.ir_type).width;

                if (!is_union && size != field_size) || size > self.avx_level.native_vector_size() {
                    pair.low = RegClass::Memory;
                    self.post_merge(size, pair);
                    return pair;
                }
            }

            let canonical_alignment =
                BitUnits::from(ctx.type_layout_cache.get(&field.ir_type).alignment);
            let is_in_memory = !(*offset % canonical_alignment).is_zero();

            if !field.is_bitfield() && is_in_memory {
                pair.low = RegClass::Memory;
                self.post_merge(size, pair);
                return pair;
            }

            let field_pair = if field.is_bitfield() {
                todo!("bitfields are not supported yet in SysV::classify_record");
            } else {
                self.classify(ctx, abi, &field.ir_type, offset_base, is_required, false)
            };

            pair.merge_with(field_pair);
            if field_pair.low.is_memory() || field_pair.high.is_memory() {
                break;
            }
        }

        self.post_merge(size, pair);
        pair
    }

    fn classify_reg_call_struct_type(
        &self,
        ctx: &BackendCtx,
        abi: &Itanium,
        struct_type: &ir::Type,
    ) -> Result<Requirement, BackendError> {
        let Some(info) = RecordInfo::try_from_type(struct_type, ctx.ir_module) else {
            panic!("SysV::classify_reg_call_struct_type is only valid for composite types");
        };

        let mut needed = RegCount::zeros();
        let mut max_vector_width = ByteUnits::of(0);

        if struct_type.has_flexible_array_member() {
            // TODO: This might not be correct. We're skipping the process of calculating the maximum vector width here.
            // This shouldn't impact anything yet, as we don't support flexible array members right
            // now, but this could be an issue later.

            let abi_type = Self::get_indirect_return_result(&ctx.type_layout_cache, struct_type);
            return Ok(Requirement::new(
                abi_type,
                RegCount::zeros(),
                ByteUnits::of(0),
            ));
        }

        // NOTE: We don't support C++ classes here yet

        for field in info.fields {
            if field.ir_type.is_product_type() {
                let requirement = self.classify_reg_call_struct_type(ctx, abi, &field.ir_type)?;

                if requirement.abi_type.is_indirect() {
                    let abi_type =
                        Self::get_indirect_return_result(&ctx.type_layout_cache, struct_type);

                    return Ok(Requirement::new(
                        abi_type,
                        RegCount::zeros(),
                        requirement.max_vector_width,
                    ));
                }
            } else {
                let requirement = self.classify_argument_type(
                    ctx,
                    abi,
                    &field.ir_type,
                    RegCount::unlimited(),
                    true,
                    true,
                )?;

                if requirement.abi_type.is_indirect() {
                    let needed = RegCount::zeros();
                    let abi_type =
                        Self::get_indirect_return_result(&ctx.type_layout_cache, struct_type);

                    return Ok(Requirement::new(
                        abi_type,
                        needed,
                        max_vector_width.max(requirement.max_vector_width),
                    ));
                }

                let inner = if let ir::Type::FixedArray(fixed_array) = &field.ir_type {
                    &fixed_array.inner
                } else {
                    &field.ir_type
                };

                if inner.is_vector() {
                    let vector_size = ctx.type_layout_cache.get(inner).width;

                    max_vector_width = max_vector_width
                        .max(vector_size)
                        .max(requirement.max_vector_width);
                }

                needed += requirement.needed;
            }
        }

        Ok(Requirement {
            abi_type: ABIType::new_direct(DirectOptions::default()),
            needed,
            max_vector_width,
        })
    }

    fn get_indirect_return_result(
        type_layout_cache: &TypeLayoutCache,
        ir_type: &ir::Type,
    ) -> ABIType {
        // NOTE: We don't support bitint types here yet

        if is_aggregate_type_for_abi(ir_type) {
            ABIType::new_indirect_natural_align(type_layout_cache, ir_type, None, None, None)
        } else if is_promotable_integer_type_for_abi(ir_type) {
            ABIType::new_extend(ir_type, None, ExtendOptions::default())
        } else {
            ABIType::new_direct(DirectOptions::default())
        }
    }

    pub fn get_indirect_result(
        &self,
        ctx: &BackendCtx,
        abi: &Itanium,
        ir_type: &ir::Type,
        free: RegCount,
    ) -> ABIType {
        // NOTE: We don't support bit-int types here yet

        if !is_aggregate_type_for_abi(ir_type) && !self.is_illegal_vector_type(ir_type) {
            if is_promotable_integer_type_for_abi(ir_type) {
                return ABIType::new_extend(ir_type, None, ExtendOptions::default());
            } else {
                return ABIType::new_direct(DirectOptions::default());
            }
        }

        let record_arg_abi = abi.get_record_arg_abi(ir_type);

        if !record_arg_abi.is_default() {
            let byval = Some(record_arg_abi.is_direct_in_memory());

            return ABIType::new_indirect_natural_align(
                &ctx.type_layout_cache,
                ir_type,
                byval,
                None,
                None,
            );
        }

        let type_layout = ctx.type_layout_cache.get(ir_type);
        let byval_alignment = type_layout.alignment;

        if !free.has_ints(1) {
            let size = type_layout.width;

            if byval_alignment == ByteUnits::of(8) && size <= ByteUnits::of(8) {
                return ABIType::new_direct(DirectOptions {
                    coerce_to_type: Some(LLVMTypeRef::new_int(size)),
                    ..DirectOptions::default()
                });
            }
        }

        ABIType::new_indirect(
            byval_alignment,
            None,
            None,
            None,
            IndirectOptions::default(),
        )
    }

    fn bits_contain_no_user_data_in_record(
        ctx: &BackendCtx,
        start_bit: BitUnits,
        end_bit: BitUnits,
        info: &RecordInfo,
    ) -> bool {
        let record_layout = ItaniumRecordLayoutBuilder::generate(
            &ctx.type_layout_cache,
            ctx.type_layout_cache.diagnostics,
            info,
            None,
        );

        assert_eq!(record_layout.field_offsets.len(), info.fields.len());

        // NOTE: We don't support C++ records here
        for (field_offset, field) in record_layout
            .field_offsets
            .iter()
            .copied()
            .zip(info.fields.iter())
        {
            if field_offset >= end_bit {
                break;
            }

            let field_start = if field_offset < start_bit {
                start_bit - field_offset
            } else {
                BitUnits::of(0)
            };

            let field_end = end_bit - field_offset;

            if !Self::bits_contain_no_user_data(ctx, &field.ir_type, field_start, field_end) {
                return false;
            }
        }

        true
    }

    fn bits_contain_no_user_data(
        ctx: &BackendCtx,
        ir_type: &ir::Type,
        start_bit: BitUnits,
        end_bit: BitUnits,
    ) -> bool {
        let size = ctx.type_layout_cache.get(ir_type).width;

        if size.to_bits() <= start_bit {
            return true;
        }

        match ir_type {
            ir::Type::Union(_) | ir::Type::Structure(_) | ir::Type::AnonymousComposite(_) => {
                // record

                Self::bits_contain_no_user_data_in_record(
                    ctx,
                    start_bit,
                    end_bit,
                    &RecordInfo::try_from_type(ir_type, ctx.ir_module)
                        .expect("failed to get record info for SysV::bits_contain_no_user_data"),
                )
            }
            ir::Type::FixedArray(fixed_array) => {
                let element_type = &fixed_array.inner;
                let element_size = ctx.type_layout_cache.get(&element_type).width;
                let num_elements = fixed_array.length;

                for i in 0..num_elements {
                    let element_offset = element_size.to_bits() * i;
                    if element_offset >= end_bit {
                        break;
                    }

                    let element_start = if element_offset < start_bit {
                        start_bit - element_offset
                    } else {
                        BitUnits::of(0)
                    };

                    let element_end = end_bit - element_offset;

                    if !Self::bits_contain_no_user_data(
                        ctx,
                        element_type,
                        element_start,
                        element_end,
                    ) {
                        return false;
                    }
                }

                true
            }
            _ => false,
        }
    }

    fn get_sse_type_at_offset(
        ctx: &BackendCtx,
        llvm_type: LLVMTypeRef,
        llvm_offset: ByteUnits,
        source_type: &ir::Type,
        source_offset: ByteUnits,
    ) -> LLVMTypeRef {
        let source_size = ctx.type_layout_cache.get(source_type).width - source_offset;

        let t0 = Self::get_fp_type_at_offset(ctx, llvm_type, llvm_offset);

        let Some(t0) = t0 else {
            return unsafe { LLVMDoubleType() };
        };

        if t0.is_double() {
            return unsafe { LLVMDoubleType() };
        }

        let t0_size = ctx.target_data.abi_size_of_type(t0);

        let t1 = if source_size > t0_size {
            Self::get_fp_type_at_offset(ctx, llvm_type, llvm_offset + t0_size)
        } else {
            None::<LLVMTypeRef>
        };

        let Some(t1) = t1 else {
            return t0;
        };

        if t0.is_float() && t1.is_float() {
            return unsafe { LLVMVectorType(t0, 2 as _) };
        }

        unsafe { LLVMDoubleType() }
    }

    fn get_fp_type_at_offset(
        ctx: &BackendCtx,
        llvm_type: LLVMTypeRef,
        llvm_offset: ByteUnits,
    ) -> Option<LLVMTypeRef> {
        if llvm_offset.is_zero() && llvm_type.is_floating_point() {
            return Some(llvm_type);
        }

        if llvm_type.is_struct() {
            if llvm_type.num_fields() == 0 {
                return None;
            }

            let element_index = unsafe {
                LLVMElementAtOffset(ctx.target_data.get(), llvm_type, llvm_offset.bytes())
            };

            let element_offset = llvm_offset
                - ByteUnits::of(unsafe {
                    LLVMOffsetOfElement(ctx.target_data.get(), llvm_type, element_index)
                });

            let element_type = llvm_type.field_types()[usize::try_from(element_index).unwrap()];

            return Self::get_fp_type_at_offset(ctx, element_type, element_offset);
        }

        if llvm_type.is_array() {
            let element_type = llvm_type.element_type();
            let element_size = ctx.target_data.abi_size_of_type(element_type);
            let element_offset = llvm_offset - element_size * (llvm_offset / element_size);
            return Self::get_fp_type_at_offset(ctx, element_type, element_offset);
        }

        None
    }

    fn pass_int128_vectors_in_mem(&self) -> bool {
        self.os.is_linux() || self.os.is_bsd()
    }

    fn get_byte_vector_type(
        &self,
        ctx: &BackendCtx,
        ir_type: &ir::Type,
    ) -> Result<LLVMTypeRef, BackendError> {
        let ir_type = if let Some(single_element_type) = is_single_element_struct(ctx, ir_type) {
            single_element_type
        } else {
            ir_type
        };

        let llvm_type = unsafe { to_backend_type(ctx.for_making_type(), ir_type)? };

        if llvm_type.is_vector() {
            if self.pass_int128_vectors_in_mem() && llvm_type.element_type().is_i128() {
                let size = ctx.type_layout_cache.get(ir_type).width;

                return Ok(unsafe {
                    LLVMVectorType(
                        LLVMTypeRef::new_int(BitUnits::of(64)),
                        (size / ByteUnits::of(8)).try_into().unwrap(),
                    )
                });
            }

            return Ok(llvm_type);
        }

        let size = ctx.type_layout_cache.get(ir_type).width;

        assert!(matches!(size.bytes(), 16 | 32 | 64));

        Ok(unsafe {
            LLVMVectorType(
                LLVMDoubleType(),
                (size / ByteUnits::of(8)).try_into().unwrap(),
            )
        })
    }

    fn get_integer_type_at_offset(
        ctx: &BackendCtx,
        llvm_type: LLVMTypeRef,
        llvm_offset: ByteUnits,
        source_type: &ir::Type,
        source_offset: ByteUnits,
    ) -> LLVMTypeRef {
        if llvm_offset.is_zero() {
            if llvm_type.is_pointer() || llvm_type.is_i64() {
                return llvm_type;
            }

            if llvm_type.is_i8() || llvm_type.is_i16() || llvm_type.is_i32() {
                let bit_width = llvm_type.integer_width();

                if Self::bits_contain_no_user_data(
                    ctx,
                    source_type,
                    source_offset.to_bits() + bit_width,
                    source_offset.to_bits() + BitUnits::of(64),
                ) {
                    return llvm_type;
                }
            }
        }

        if llvm_type.is_struct() {
            if llvm_offset < ctx.target_data.abi_size_of_type(llvm_type) {
                let field_index = unsafe {
                    LLVMElementAtOffset(ctx.target_data.get(), llvm_type, llvm_offset.bytes())
                };

                let llvm_offset = llvm_offset
                    - ByteUnits::of(unsafe {
                        LLVMOffsetOfElement(ctx.target_data.get(), llvm_type, field_index)
                    });

                let llvm_element_type =
                    llvm_type.field_types()[usize::try_from(field_index).unwrap()];

                return Self::get_integer_type_at_offset(
                    ctx,
                    llvm_element_type,
                    llvm_offset,
                    source_type,
                    source_offset,
                );
            }
        }

        if llvm_type.is_array() {
            let element_type = llvm_type.element_type();
            let element_size = ctx.target_data.abi_size_of_type(element_type);
            let element_offset = element_size * (llvm_offset / element_size);

            return Self::get_integer_type_at_offset(
                ctx,
                element_type,
                llvm_offset - element_offset,
                source_type,
                source_offset,
            );
        }

        let type_size = ctx.type_layout_cache.get(source_type).width;
        assert_ne!(type_size, source_offset);

        LLVMTypeRef::new_int(ByteUnits::of(8).min(type_size - source_offset))
    }

    fn make_byval_argument_pair(
        ctx: &BackendCtx,
        low: LLVMTypeRef,
        high: LLVMTypeRef,
    ) -> LLVMTypeRef {
        let mut low = low;
        let low_size = ctx.target_data.abi_size_of_type(low);
        let high_align =
            ByteUnits::of(unsafe { LLVMABIAlignmentOfType(ctx.target_data.get(), high).into() });
        let high_start = low_size.align_to(high_align);

        assert!(!high_start.is_zero() && high_start <= ByteUnits::of(8));

        if high_start != ByteUnits::of(8) {
            if low.is_float() {
                low = unsafe { LLVMDoubleType() };
            } else {
                assert!(low.is_integer_or_pointer());
                low = unsafe { LLVMInt64Type() };
            }
        }

        let mut element_types = [low, high];
        let result = unsafe { LLVMStructType(element_types.as_mut_ptr(), 2 as _, false as _) };

        assert_eq!(
            unsafe { LLVMOffsetOfElement(ctx.target_data.get(), result, 1) },
            8,
            "invalid x86_64 argument pair"
        );

        result
    }

    fn is_illegal_vector_type(&self, ty: &ir::Type) -> bool {
        if ty.is_vector() {
            todo!("SysV::is_illegal_vector_type")
        }

        false
    }
}

#[derive(Clone, Debug)]
struct Requirement {
    pub abi_type: ABIType,
    pub needed: RegCount,
    pub max_vector_width: ByteUnits,
}

impl Requirement {
    pub fn new(abi_type: ABIType, needed: RegCount, max_vector_width: ByteUnits) -> Self {
        Self {
            abi_type,
            needed,
            max_vector_width,
        }
    }
}

fn is_single_element_struct<'a>(
    ctx: &'a BackendCtx,
    ir_type: &'a ir::Type,
) -> Option<&'a ir::Type> {
    if ir_type.has_flexible_array_member() {
        return None;
    }

    let Some(record_info) = RecordInfo::try_from_type(ir_type, ctx.ir_module) else {
        return None;
    };

    // NOTE: We don't support C++ records here yet

    let mut found = None::<&ir::Type>;

    for field in record_info.fields.iter() {
        let mut field_type = &field.ir_type;

        if is_empty_field(
            field_type,
            ctx.ir_module,
            IsEmptyRecordOptions {
                allow_arrays: true,
                ..Default::default()
            },
        ) {
            continue;
        }

        if found.is_some() {
            // We have multiple non-empty fields
            return None;
        }

        while let ir::Type::FixedArray(fixed_array) = field_type {
            if fixed_array.length == 1 {
                field_type = &fixed_array.inner;
            } else {
                break;
            }
        }

        if !is_aggregate_type_for_abi(field_type) {
            found = Some(field_type);
        } else {
            found = is_single_element_struct(ctx, field_type);

            if found.is_none() {
                return None;
            }
        }
    }

    if let Some(found) = found {
        if ctx.type_layout_cache.get(found).width == ctx.type_layout_cache.get(ir_type).width {
            return Some(found);
        }
    }

    None
}
