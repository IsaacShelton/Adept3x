use crate::{
    data_units::ByteUnits,
    ir,
    llvm_backend::{
        abi::{
            abi_function::{ABIFunction, ABIParam},
            abi_type::{ABITypeKind, InAlloca, Indirect},
            has_scalar_evaluation_kind,
        },
        builder::Builder,
        ctx::{Address, BackendCtx, FunctionSkeleton, RawAddress},
        target_data::TargetData,
    },
    target_info::type_layout::TypeLayoutCache,
};
use cstr::cstr;
use llvm_sys::{
    core::{
        LLVMBuildAlloca, LLVMBuildArrayAlloca, LLVMBuildBitCast, LLVMBuildLoad2, LLVMBuildStore,
        LLVMBuildStructGEP2, LLVMDumpModule, LLVMGetInsertBlock, LLVMGetLastParam, LLVMGetParam,
        LLVMGetTypeKind, LLVMGetUndef, LLVMInt32Type, LLVMPositionBuilderAtEnd,
        LLVMPositionBuilderBefore, LLVMSetAlignment,
    },
    prelude::{LLVMBasicBlockRef, LLVMTypeRef, LLVMValueRef},
    target::LLVMPreferredAlignmentOfType,
    LLVMTypeKind,
};
use std::ffi::CStr;

pub struct BackendFnCtx {
    pub return_location: Option<ReturnLocation>,
}

pub struct ReturnLocation {
    pub return_value_address: Address,
    pub return_value_address_pointer: Option<Address>,
}

impl ReturnLocation {
    pub fn indirect(
        builder: &Builder,
        target_data: &TargetData,
        type_layout_cache: &TypeLayoutCache,
        abi_function: &ABIFunction,
        function: LLVMValueRef,
        indirect: &Indirect,
        alloca_insertion_point: LLVMValueRef,
        return_type: LLVMTypeRef,
    ) -> Self {
        let sret_argument = unsafe { LLVMGetParam(function, indirect.sret_position().into()) };

        let value = make_natural_address_for_pointer(
            type_layout_cache,
            sret_argument,
            &abi_function.return_type.ir_type,
            Some(indirect.align),
        );

        let pointer = (!indirect.byval).then(|| {
            let pointer = create_default_align_tmp_alloca(
                target_data,
                builder,
                alloca_insertion_point,
                return_type,
                cstr!("result.ptr"),
            );
            unsafe { LLVMBuildStore(builder.get(), value.base_pointer(), pointer.base_pointer()) };
            pointer
        });

        ReturnLocation {
            return_value_address: value,
            return_value_address_pointer: pointer.map(Into::into),
        }
    }

    pub fn inalloca(
        builder: &Builder,
        ctx: &BackendCtx,
        skeleton: &FunctionSkeleton,
        inalloca: &InAlloca,
        abi_return_info: &ABIParam,
    ) -> Self {
        let last_argument = unsafe { LLVMGetLastParam(skeleton.function) };

        let inalloca_combined_struct = skeleton
            .abi_function
            .as_ref()
            .unwrap()
            .inalloca_combined_struct
            .unwrap();

        let address = unsafe {
            LLVMBuildStructGEP2(
                builder.get(),
                inalloca_combined_struct,
                last_argument,
                inalloca.alloca_field_index,
                cstr!("").as_ptr(),
            )
        };

        let pointer = Address {
            base: RawAddress {
                base: address,
                nullable: false,
                alignment: ctx.target_data.pointer_alignment(),
            },
            offset: None,
        };

        let addr = unsafe {
            let load = LLVMBuildLoad2(
                builder.get(),
                inalloca_combined_struct,
                address,
                cstr!("agg.result").as_ptr(),
            );

            LLVMSetAlignment(
                load,
                ctx.target_data
                    .pointer_alignment()
                    .bytes()
                    .try_into()
                    .unwrap(),
            );

            load
        };

        let value = Address {
            base: RawAddress {
                base: addr,
                nullable: false,
                alignment: get_natural_type_alignment(
                    &ctx.type_layout_cache,
                    &abi_return_info.ir_type,
                ),
            },
            offset: None,
        };

        ReturnLocation {
            return_value_address: value,
            return_value_address_pointer: Some(pointer),
        }
    }

    pub fn normal(
        builder: &Builder,
        type_layout_cache: &TypeLayoutCache,
        alloca_insertion_point: LLVMValueRef,
        return_ir_type: &ir::Type,
        return_type: LLVMTypeRef,
    ) -> Self {
        ReturnLocation {
            return_value_address: create_ir_tmp(
                builder,
                type_layout_cache,
                alloca_insertion_point,
                return_ir_type,
                return_type,
                Some(cstr!("retval")),
            )
            .into(),
            return_value_address_pointer: None,
        }
    }
}

fn create_default_align_tmp_alloca(
    target_data: &TargetData,
    builder: &Builder,
    alloca_insertion_point: LLVMValueRef,
    ty: LLVMTypeRef,
    name: &CStr,
) -> RawAddress {
    let alignment = ByteUnits::from(unsafe { LLVMPreferredAlignmentOfType(target_data.get(), ty) });
    create_tmp_alloca_address(builder, alloca_insertion_point, ty, alignment, name, None)
}

fn create_ir_tmp(
    builder: &Builder,
    type_layout_cache: &TypeLayoutCache,
    alloca_insertion_point: LLVMValueRef,
    ir_type: &ir::Type,
    ty: LLVMTypeRef,
    name: Option<&CStr>,
) -> RawAddress {
    let alignment = type_layout_cache.get(ir_type).alignment;

    create_tmp_alloca_address(
        builder,
        alloca_insertion_point,
        ty,
        alignment,
        name.unwrap_or_else(|| cstr!("tmp")),
        None,
    )
}

fn create_tmp_alloca_address(
    builder: &Builder,
    alloca_insertion_point: LLVMValueRef,
    ty: LLVMTypeRef,
    alignment: ByteUnits,
    name: &CStr,
    array_size: Option<LLVMValueRef>,
) -> RawAddress {
    let alloca = create_tmp_alloca_inst(builder, ty, name, array_size, alloca_insertion_point);
    unsafe { LLVMSetAlignment(alloca, alignment.bytes().try_into().unwrap()) };

    RawAddress {
        base: alloca,
        nullable: false,
        alignment,
    }
}

fn create_tmp_alloca_inst(
    builder: &Builder,
    ty: LLVMTypeRef,
    name: &CStr,
    array_size: Option<LLVMValueRef>,
    alloca_insertion_point: LLVMValueRef,
) -> LLVMValueRef {
    let alloca = if let Some(array_size) = array_size {
        unsafe {
            let current_block = LLVMGetInsertBlock(builder.get());
            LLVMPositionBuilderBefore(builder.get(), alloca_insertion_point);

            let inserted = LLVMBuildArrayAlloca(builder.get(), ty, array_size, name.as_ptr());
            LLVMPositionBuilderAtEnd(builder.get(), current_block);
            inserted
        }
    } else {
        unsafe { LLVMBuildAlloca(builder.get(), ty, name.as_ptr()) }
    };

    alloca
}

fn make_natural_address_for_pointer(
    type_layout_cache: &TypeLayoutCache,
    ptr: LLVMValueRef,
    ir_type: &ir::Type,
    alignment: Option<ByteUnits>,
) -> Address {
    let alignment = match alignment {
        Some(ByteUnits::ZERO) | None => get_natural_type_alignment(type_layout_cache, ir_type),
        Some(alignment) => alignment,
    };

    Address {
        base: RawAddress {
            base: ptr,
            nullable: false,
            alignment,
        },
        offset: None,
    }
}

pub fn get_natural_type_alignment(
    type_layout_cache: &TypeLayoutCache,
    ir_type: &ir::Type,
) -> ByteUnits {
    type_layout_cache.get(ir_type).alignment
}

pub fn emit_prologue(
    ctx: &BackendCtx,
    skeleton: &FunctionSkeleton,
    builder: &Builder,
    entry_basicblock: LLVMBasicBlockRef,
) -> Option<BackendFnCtx> {
    let abi_function = skeleton.abi_function.as_ref()?;
    let abi_return_info = &abi_function.return_type;
    let return_type = skeleton.function_type.return_type;
    let is_return_void = unsafe { LLVMGetTypeKind(return_type) } == LLVMTypeKind::LLVMVoidTypeKind;

    unsafe { LLVMPositionBuilderAtEnd(builder.get(), entry_basicblock) };

    let undef = unsafe { LLVMGetUndef(LLVMInt32Type()) };
    let alloca_insertion_point = unsafe {
        LLVMBuildBitCast(
            builder.get(),
            undef,
            LLVMInt32Type(),
            cstr!("allocapt").as_ptr(),
        )
    };

    let return_location = (!is_return_void).then(|| match &abi_return_info.abi_type.kind {
        ABITypeKind::Indirect(indirect) => ReturnLocation::indirect(
            builder,
            &ctx.target_data,
            &ctx.type_layout_cache,
            &abi_function,
            skeleton.function,
            indirect,
            alloca_insertion_point,
            return_type,
        ),
        ABITypeKind::InAlloca(inalloca)
            if !has_scalar_evaluation_kind(&abi_return_info.ir_type) =>
        {
            ReturnLocation::inalloca(builder, ctx, skeleton, inalloca, abi_return_info)
        }
        _ => ReturnLocation::normal(
            builder,
            &ctx.type_layout_cache,
            alloca_insertion_point,
            &abi_return_info.ir_type,
            return_type,
        ),
    });

    unsafe { LLVMDumpModule(ctx.backend_module.get()) };
    unimplemented!();
}
