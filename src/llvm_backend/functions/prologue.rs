use cstr::cstr;
use llvm_sys::{
    core::{
        LLVMBuildAlloca, LLVMBuildArrayAlloca, LLVMBuildBitCast, LLVMBuildStore, LLVMDumpModule,
        LLVMGetInsertBlock, LLVMGetParam, LLVMGetTypeKind, LLVMGetUndef, LLVMInt32Type,
        LLVMPositionBuilderAtEnd, LLVMPositionBuilderBefore, LLVMSetAlignment,
    },
    prelude::{LLVMBasicBlockRef, LLVMTypeRef, LLVMValueRef},
    target::LLVMPreferredAlignmentOfType,
    LLVMTypeKind,
};
use std::ffi::CStr;

use crate::{
    data_units::ByteUnits,
    ir,
    llvm_backend::{
        abi::{
            abi_function::ABIFunction,
            abi_type::{ABITypeKind, Indirect},
        },
        builder::Builder,
        ctx::{Address, BackendCtx, FunctionSkeleton, RawAddress},
        target_data::TargetData,
    },
    target_info::type_layout::TypeLayoutCache,
};

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
    ) -> Option<ReturnLocation> {
        let sret_argument = unsafe { LLVMGetParam(function, indirect.sret_position().into()) };

        let value = make_natural_address_for_pointer(
            type_layout_cache,
            sret_argument,
            &abi_function.return_type.ir_type,
            Some(indirect.align),
        );

        let pointer = (!indirect.byval).then(|| {
            let pointer = create_default_align_tmp_alloca(
                builder,
                target_data,
                return_type,
                cstr!("result.ptr"),
                alloca_insertion_point,
            );
            unsafe { LLVMBuildStore(builder.get(), value.base_pointer(), pointer.base_pointer()) };
            pointer
        });

        Some(ReturnLocation {
            return_value_address: value,
            return_value_address_pointer: pointer.map(Into::into),
        })
    }
}

fn create_default_align_tmp_alloca(
    builder: &Builder,
    target_data: &TargetData,
    ty: LLVMTypeRef,
    name: &CStr,
    alloca_insertion_point: LLVMValueRef,
) -> RawAddress {
    let align = ByteUnits::from(unsafe { LLVMPreferredAlignmentOfType(target_data.get(), ty) });

    create_tmp_alloca_address(builder, ty, align, name, None, alloca_insertion_point)
}

fn create_tmp_alloca_address(
    builder: &Builder,
    ty: LLVMTypeRef,
    alignment: ByteUnits,
    name: &CStr,
    array_size: Option<LLVMValueRef>,
    alloca_insertion_point: LLVMValueRef,
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
        let current_block = unsafe { LLVMGetInsertBlock(builder.get()) };

        unsafe { LLVMPositionBuilderBefore(builder.get(), alloca_insertion_point) };
        let inserted =
            unsafe { LLVMBuildArrayAlloca(builder.get(), ty, array_size, name.as_ptr()) };
        unsafe { LLVMPositionBuilderAtEnd(builder.get(), current_block) };

        inserted
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
        ABITypeKind::Direct(_) => todo!(),
        ABITypeKind::Extend(_) => todo!(),
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
        ABITypeKind::IndirectAliased(_) => todo!(),
        ABITypeKind::Ignore => todo!(),
        ABITypeKind::Expand(_) => todo!(),
        ABITypeKind::CoerceAndExpand(_) => todo!(),
        ABITypeKind::InAlloca(_) => todo!(),
    });

    unsafe { LLVMDumpModule(ctx.backend_module.get()) };
    unimplemented!();
}
