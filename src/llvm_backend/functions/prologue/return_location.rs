use super::helpers::{
    build_default_align_tmp_alloca, build_tmp, get_natural_type_alignment,
    make_natural_address_for_pointer,
};
use crate::{
    ir,
    llvm_backend::{
        abi::abi_type::{InAlloca, Indirect},
        address::Address,
        backend_type::to_backend_type,
        builder::{build_aligned_load, Builder},
        ctx::{BackendCtx, FunctionSkeleton},
        error::BackendError,
        raw_address::RawAddress,
    },
};
use cstr::cstr;
use llvm_sys::{
    core::{LLVMBuildStore, LLVMBuildStructGEP2, LLVMGetLastParam, LLVMGetParam},
    prelude::{LLVMTypeRef, LLVMValueRef},
};

pub struct ReturnLocation {
    pub return_value_address: Address,
    pub return_value_address_pointer: Option<Address>,
}

impl ReturnLocation {
    pub fn indirect(
        ctx: &BackendCtx,
        builder: &Builder,
        ir_function: &ir::Function,
        function: LLVMValueRef,
        indirect: &Indirect,
        alloca_insertion_point: LLVMValueRef,
    ) -> Result<Self, BackendError> {
        let target_data = &ctx.target_data;
        let sret_argument = unsafe { LLVMGetParam(function, indirect.sret_position().into()) };

        let value = make_natural_address_for_pointer(
            ctx,
            sret_argument,
            &ir_function.return_type,
            Some(indirect.align),
            None,
        )?;

        let pointer = (!indirect.byval).then(|| {
            let pointer = build_default_align_tmp_alloca(
                target_data,
                builder,
                alloca_insertion_point,
                value.pointer_type(),
                cstr!("result.ptr"),
            );
            unsafe { LLVMBuildStore(builder.get(), value.base_pointer(), pointer.base_pointer()) };
            pointer
        });

        Ok(ReturnLocation {
            return_value_address: value,
            return_value_address_pointer: pointer.map(Into::into),
        })
    }

    pub fn inalloca(
        ctx: &BackendCtx,
        builder: &Builder,
        skeleton: &FunctionSkeleton,
        inalloca: &InAlloca,
        inalloca_subtypes: &[LLVMTypeRef],
    ) -> Result<Self, BackendError> {
        let ir_function = ctx
            .ir_module
            .functions
            .get(&skeleton.ir_function_ref)
            .unwrap();

        let last_argument = unsafe { LLVMGetLastParam(skeleton.function) };

        let inalloca_combined_struct = skeleton
            .abi_function
            .as_ref()
            .unwrap()
            .inalloca_combined_struct
            .as_ref()
            .unwrap();

        let index = inalloca.alloca_field_index;

        let address = unsafe {
            LLVMBuildStructGEP2(
                builder.get(),
                inalloca_combined_struct.ty,
                last_argument,
                index,
                cstr!("").as_ptr(),
            )
        };

        let pointer_alignment = ctx.ir_module.target_info.pointer_layout().alignment;

        let pointer = Address {
            base: RawAddress {
                base: address,
                nullable: false,
                alignment: pointer_alignment,
                element_type: inalloca_subtypes[usize::try_from(index).unwrap()],
            },
            offset: None,
        };

        let addr = build_aligned_load(
            builder,
            inalloca_combined_struct.ty,
            address,
            pointer_alignment,
            cstr!("agg.result"),
        );

        let value = Address {
            base: RawAddress {
                base: addr,
                nullable: false,
                alignment: get_natural_type_alignment(
                    &ctx.type_layout_cache,
                    &ir_function.return_type,
                ),
                element_type: unsafe {
                    to_backend_type(ctx.for_making_type(), &ir_function.return_type)?
                },
            },
            offset: None,
        };

        Ok(ReturnLocation {
            return_value_address: value,
            return_value_address_pointer: Some(pointer),
        })
    }

    pub fn normal(
        builder: &Builder,
        ctx: &BackendCtx,
        alloca_insertion_point: LLVMValueRef,
        return_ir_type: &ir::Type,
    ) -> Result<Self, BackendError> {
        let raw_address = build_tmp(
            builder,
            ctx,
            alloca_insertion_point,
            return_ir_type,
            Some(cstr!("retval")),
        )?;

        Ok(ReturnLocation {
            return_value_address: raw_address.into(),
            return_value_address_pointer: None,
        })
    }
}
