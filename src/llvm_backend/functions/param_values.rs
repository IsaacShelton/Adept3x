use super::{param_value::ParamValue, prologue::helpers::build_tmp_alloca_address};
use crate::{
    data_units::ByteUnits,
    ir,
    llvm_backend::{
        abi::{abi_type::InAlloca, has_scalar_evaluation_kind},
        address::Address,
        backend_type::{to_backend_mem_type, to_backend_type},
        builder::{build_load, build_struct_gep, Builder},
        ctx::{BackendCtx, FunctionSkeleton},
        error::BackendError,
        functions::prologue::helpers::make_natural_address_for_pointer,
        raw_address::RawAddress,
    },
    target_info::type_layout::TypeLayoutCache,
};
use cstr::cstr;
use llvm_sys::{
    core::{
        LLVMBuildMemCpy, LLVMBuildTruncOrBitCast, LLVMConstInt, LLVMGetParam, LLVMGetUndef,
        LLVMGetValueKind, LLVMInt1Type, LLVMIsThreadLocal,
    },
    prelude::{LLVMTypeRef, LLVMValueRef},
    target::LLVMIntPtrType,
    LLVMValueKind,
};
use std::{ffi::CStr, ops::Range};

pub struct ParamValues {
    values: Vec<ParamValue>,
}

impl ParamValues {
    pub fn new() -> Self {
        Self {
            values: Vec::<ParamValue>::with_capacity(16),
        }
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = &ParamValue> {
        self.values.iter()
    }

    #[allow(clippy::too_many_arguments)]
    pub fn push_inalloca(
        &mut self,
        builder: &Builder,
        ctx: &BackendCtx,
        inalloca: &InAlloca,
        llvm_param_range: Range<usize>,
        arg_struct: &Address,
        ty: &ir::Type,
        type_layout_cache: &TypeLayoutCache,
        inalloca_subtypes: &[LLVMTypeRef],
    ) -> Result<(), BackendError> {
        assert_eq!(llvm_param_range.count(), 0);

        let field_index = inalloca.alloca_field_index;

        let mut value = build_struct_gep(
            builder,
            ctx.target_data,
            arg_struct,
            field_index as usize,
            Some(inalloca_subtypes),
        );

        if inalloca.indirect {
            value = Address::from(RawAddress {
                base: build_load(builder, &value, false),
                nullable: false,
                alignment: type_layout_cache.get(ty).alignment,
                element_type: unsafe {
                    to_backend_mem_type(ctx.for_making_type(), type_layout_cache, ty, false)?
                },
            });
        }

        self.values.push(ParamValue::Indirect(value));
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn push_indirect(
        &mut self,
        builder: &Builder,
        ctx: &BackendCtx,
        skeleton: &FunctionSkeleton,
        mut llvm_param_range: Range<usize>,
        ty: &ir::Type,
        indirect_alignment: ByteUnits,
        realign: bool,
        indirect_aliased: bool,
        alloca_insertion_point: LLVMValueRef,
    ) -> Result<(), BackendError> {
        assert_eq!(llvm_param_range.clone().count(), 1);
        let index = llvm_param_range.next().unwrap();

        let argument = unsafe { LLVMGetParam(skeleton.function, index.try_into().unwrap()) };

        let mut param_address =
            make_natural_address_for_pointer(ctx, argument, ty, Some(indirect_alignment), false)?;

        if has_scalar_evaluation_kind(ty) {
            let value = emit_load_of_scalar(builder, &param_address, false, ty);
            self.values.push(ParamValue::Direct(value));
            return Ok(());
        }

        if realign || indirect_aliased {
            let aligned_tmp =
                build_mem_tmp(ctx, builder, alloca_insertion_point, ty, cstr!("coerce"))?;
            let size = ctx.type_layout_cache.get(ty).width;

            let pointer_sized_int_ty = unsafe { LLVMIntPtrType(ctx.target_data.get()) };

            let num_bytes =
                unsafe { LLVMConstInt(pointer_sized_int_ty, size.bytes(), false as i32) };

            unsafe {
                LLVMBuildMemCpy(
                    builder.get(),
                    aligned_tmp.base_pointer(),
                    aligned_tmp.alignment.bytes().try_into().unwrap(),
                    param_address.base_pointer(),
                    param_address.base.alignment.bytes().try_into().unwrap(),
                    num_bytes,
                );
            }

            param_address = aligned_tmp.into();
        }

        self.values.push(ParamValue::Indirect(param_address));
        Ok(())
    }

    pub fn push_ignore(
        &mut self,
        builder: &Builder,
        ctx: &BackendCtx,
        llvm_param_range: Range<usize>,
        ty: &ir::Type,
        alloca_insertion_point: LLVMValueRef,
    ) -> Result<(), BackendError> {
        assert_eq!(llvm_param_range.count(), 0);

        if has_scalar_evaluation_kind(ty) {
            let scalar_ty = unsafe { to_backend_type(ctx.for_making_type(), ty)? };
            let undef = unsafe { LLVMGetUndef(scalar_ty) };
            self.values.push(ParamValue::Direct(undef));
        } else {
            let tmp = build_mem_tmp(ctx, builder, alloca_insertion_point, ty, cstr!("tmp"))?;
            self.values.push(ParamValue::Indirect(tmp.into()));
        }

        Ok(())
    }
}

fn emit_load_of_scalar(
    builder: &Builder,
    address: &Address,
    is_volitile: bool,
    ir_type: &ir::Type,
) -> LLVMValueRef {
    let address = match unsafe { LLVMGetValueKind(address.base_pointer()) } {
        LLVMValueKind::LLVMGlobalVariableValueKind
            if unsafe { LLVMIsThreadLocal(address.base_pointer()) } != 0 =>
        {
            todo!("thread locals in emit_load_of_scalar not supported yet")
        }
        _ => address,
    };

    match ir_type {
        ir::Type::Vector(_) => todo!("vector types in emit_load_of_scalar not supported yet"),
        ir::Type::Atomic(_) => todo!("atomic types in emit_load_of_scalar not supported yet"),
        _ => (),
    }

    let load = build_load(builder, address, is_volitile);
    emit_from_mem(builder, load, ir_type)
}

fn emit_from_mem(builder: &Builder, value: LLVMValueRef, ir_type: &ir::Type) -> LLVMValueRef {
    match ir_type {
        ir::Type::Boolean => unsafe {
            LLVMBuildTruncOrBitCast(builder.get(), value, LLVMInt1Type(), cstr!("").as_ptr())
        },
        _ => value,
    }
}

fn build_mem_tmp(
    ctx: &BackendCtx,
    builder: &Builder,
    alloca_insertion_point: LLVMValueRef,
    ir_type: &ir::Type,
    name: &CStr,
) -> Result<RawAddress, BackendError> {
    let alignment = ctx.type_layout_cache.get(ir_type).alignment;
    build_mem_tmp_ex(
        ctx,
        builder,
        alloca_insertion_point,
        ir_type,
        alignment,
        name,
    )
}

fn build_mem_tmp_ex(
    ctx: &BackendCtx,
    builder: &Builder,
    alloca_insertion_point: LLVMValueRef,
    ir_type: &ir::Type,
    alignment: ByteUnits,
    name: &CStr,
) -> Result<RawAddress, BackendError> {
    let backend_type = unsafe { to_backend_type(ctx.for_making_type(), ir_type)? };

    Ok(build_tmp_alloca_address(
        builder,
        alloca_insertion_point,
        backend_type,
        alignment,
        name,
        None,
    ))
}
