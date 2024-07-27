use crate::{
    ir,
    llvm_backend::{
        address::Address,
        builder::{Builder, Volatility},
    },
};
use cstr::cstr;
use llvm_sys::{
    core::{LLVMBuildTruncOrBitCast, LLVMGetValueKind, LLVMInt1Type, LLVMIsThreadLocal},
    prelude::LLVMValueRef,
    LLVMValueKind,
};

fn is_thread_local(value: LLVMValueRef) -> bool {
    unsafe {
        LLVMGetValueKind(value) == LLVMValueKind::LLVMGlobalVariableValueKind
            && LLVMIsThreadLocal(value) != 0
    }
}

pub fn emit_load_of_scalar(
    builder: &Builder,
    address: &Address,
    volatility: Volatility,
    ir_type: &ir::Type,
) -> LLVMValueRef {
    let address = if is_thread_local(address.base_pointer()) {
        todo!("thread locals in emit_load_of_scalar not supported yet")
    } else {
        address
    };

    match ir_type {
        ir::Type::Vector(_) => todo!("vector types in emit_load_of_scalar not supported yet"),
        ir::Type::Atomic(_) => todo!("atomic types in emit_load_of_scalar not supported yet"),
        _ => (),
    }

    let load = builder.load(address, volatility);
    emit_from_mem(builder, load, ir_type)
}

pub fn emit_from_mem(builder: &Builder, value: LLVMValueRef, ir_type: &ir::Type) -> LLVMValueRef {
    match ir_type {
        ir::Type::Boolean => unsafe {
            LLVMBuildTruncOrBitCast(builder.get(), value, LLVMInt1Type(), cstr!("").as_ptr())
        },
        _ => value,
    }
}
