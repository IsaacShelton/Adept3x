use super::param_value::ParamValue;
use crate::{
    data_units::ByteUnits,
    ir,
    llvm_backend::{
        abi::{abi_type::InAlloca, has_scalar_evaluation_kind},
        address::Address,
        backend_type::to_backend_mem_type,
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
        LLVMBuildTruncOrBitCast, LLVMGetParam, LLVMGetValueKind, LLVMInt1Type, LLVMIsThreadLocal,
    },
    prelude::{LLVMTypeRef, LLVMValueRef},
    LLVMValueKind,
};
use std::ops::Range;

pub struct ParamValues {
    values: Vec<ParamValue>,
}

impl ParamValues {
    pub fn new() -> Self {
        Self {
            values: Vec::<ParamValue>::with_capacity(16),
        }
    }

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

    pub fn push_indirect(
        &mut self,
        builder: &Builder,
        ctx: &BackendCtx,
        skeleton: &FunctionSkeleton,
        mut llvm_param_range: Range<usize>,
        ty: &ir::Type,
        indirect_alignment: ByteUnits,
    ) -> Result<(), BackendError> {
        assert_eq!(llvm_param_range.clone().count(), 1);
        let index = llvm_param_range.next().unwrap();

        let argument = unsafe { LLVMGetParam(skeleton.function, index.try_into().unwrap()) };

        let param_address =
            make_natural_address_for_pointer(ctx, argument, ty, Some(indirect_alignment), false)?;

        if has_scalar_evaluation_kind(ty) {
            let value = emit_load_of_scalar(builder, &param_address, false, ty);
            self.values.push(ParamValue::Direct(value));
            return Ok(());
        }

        unimplemented!("push_indirect");
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
