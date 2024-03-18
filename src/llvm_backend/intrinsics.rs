use super::{
    builder::Builder, module::BackendModule, null_terminated_string::build_literal_cstring,
};
use crate::{
    ir::{OverflowOperation, OverflowOperator},
    resolved::{IntegerBits, IntegerSign},
};
use cstr::cstr;
use llvm_sys::{
    core::{
        LLVMAddFunction, LLVMAppendBasicBlock, LLVMBuildCall2, LLVMBuildUnreachable,
        LLVMFunctionType, LLVMGetNamedFunction, LLVMInt16Type, LLVMInt1Type, LLVMInt32Type,
        LLVMInt64Type, LLVMInt8Type, LLVMPointerType, LLVMPositionBuilderAtEnd, LLVMStructType,
        LLVMVoidType,
    },
    prelude::{LLVMBool, LLVMModuleRef, LLVMTypeRef, LLVMValueRef},
};
use memo_map::MemoMap;
use std::{
    cell::OnceCell,
    ffi::{c_uint, CStr, CString},
};

pub struct Intrinsics {
    module: LLVMModuleRef,
    memcpy: OnceCell<(LLVMValueRef, LLVMTypeRef)>,
    memset: OnceCell<(LLVMValueRef, LLVMTypeRef)>,
    stacksave: OnceCell<(LLVMValueRef, LLVMTypeRef)>,
    stackrestore: OnceCell<(LLVMValueRef, LLVMTypeRef)>,
    va_start: OnceCell<(LLVMValueRef, LLVMTypeRef)>,
    va_end: OnceCell<(LLVMValueRef, LLVMTypeRef)>,
    va_copy: OnceCell<(LLVMValueRef, LLVMTypeRef)>,
    overflow_operations: MemoMap<OverflowOperation, (LLVMValueRef, LLVMTypeRef)>,
    expect_i1: OnceCell<(LLVMValueRef, LLVMTypeRef)>,
    on_overflow: OnceCell<(LLVMValueRef, LLVMTypeRef)>,
    printf: OnceCell<(LLVMValueRef, LLVMTypeRef)>,
    abort: OnceCell<(LLVMValueRef, LLVMTypeRef)>,
}

impl Intrinsics {
    pub unsafe fn new(module: &BackendModule) -> Self {
        Self {
            module: module.get(),
            memcpy: OnceCell::new(),
            memset: OnceCell::new(),
            stacksave: OnceCell::new(),
            stackrestore: OnceCell::new(),
            va_start: OnceCell::new(),
            va_end: OnceCell::new(),
            va_copy: OnceCell::new(),
            overflow_operations: MemoMap::new(),
            expect_i1: OnceCell::new(),
            on_overflow: OnceCell::new(),
            printf: OnceCell::new(),
            abort: OnceCell::new(),
        }
    }

    pub unsafe fn memcpy(&mut self) -> (LLVMValueRef, LLVMTypeRef) {
        *self.memcpy.get_or_init(|| {
            let mut parameter_types = [
                LLVMPointerType(LLVMInt8Type(), 0),
                LLVMPointerType(LLVMInt8Type(), 0),
                LLVMInt64Type(),
                LLVMInt1Type(),
            ];
            let return_type = LLVMVoidType();
            let is_var_arg = false;

            let signature = LLVMFunctionType(
                return_type,
                parameter_types.as_mut_ptr(),
                parameter_types.len() as c_uint,
                is_var_arg as LLVMBool,
            );

            (
                LLVMAddFunction(
                    self.module,
                    cstr!("llvm.memcpy.p0i8.p0i8.i64").as_ptr(),
                    signature,
                ),
                signature,
            )
        })
    }

    pub unsafe fn overflow_operation(
        &self,
        operation: &OverflowOperation,
    ) -> (LLVMValueRef, LLVMTypeRef) {
        *self.overflow_operations.get_or_insert(&operation, || {
            let backend_type = match operation.bits {
                IntegerBits::Bits8 => LLVMInt8Type(),
                IntegerBits::Bits16 => LLVMInt16Type(),
                IntegerBits::Bits32 => LLVMInt32Type(),
                IntegerBits::Bits64 | IntegerBits::Normal => LLVMInt64Type(),
            };

            let name = match operation {
                OverflowOperation {
                    operator: OverflowOperator::Add,
                    bits: IntegerBits::Bits8,
                    sign: IntegerSign::Signed,
                } => "llvm.sadd.with.overflow.i8",
                OverflowOperation {
                    operator: OverflowOperator::Add,
                    bits: IntegerBits::Bits16,
                    sign: IntegerSign::Signed,
                } => "llvm.sadd.with.overflow.i16",
                OverflowOperation {
                    operator: OverflowOperator::Add,
                    bits: IntegerBits::Bits32,
                    sign: IntegerSign::Signed,
                } => "llvm.sadd.with.overflow.i32",
                OverflowOperation {
                    operator: OverflowOperator::Add,
                    bits: IntegerBits::Bits64 | IntegerBits::Normal,
                    sign: IntegerSign::Signed,
                } => "llvm.sadd.with.overflow.i64",
                OverflowOperation {
                    operator: OverflowOperator::Add,
                    bits: IntegerBits::Bits8,
                    sign: IntegerSign::Unsigned,
                } => "llvm.uadd.with.overflow.i8",
                OverflowOperation {
                    operator: OverflowOperator::Add,
                    bits: IntegerBits::Bits16,
                    sign: IntegerSign::Unsigned,
                } => "llvm.uadd.with.overflow.i16",
                OverflowOperation {
                    operator: OverflowOperator::Add,
                    bits: IntegerBits::Bits32,
                    sign: IntegerSign::Unsigned,
                } => "llvm.uadd.with.overflow.i32",
                OverflowOperation {
                    operator: OverflowOperator::Add,
                    bits: IntegerBits::Bits64 | IntegerBits::Normal,
                    sign: IntegerSign::Unsigned,
                } => "llvm.uadd.with.overflow.i64",
                OverflowOperation {
                    operator: OverflowOperator::Subtract,
                    bits: IntegerBits::Bits8,
                    sign: IntegerSign::Signed,
                } => "llvm.ssub.with.overflow.i8",
                OverflowOperation {
                    operator: OverflowOperator::Subtract,
                    bits: IntegerBits::Bits16,
                    sign: IntegerSign::Signed,
                } => "llvm.ssub.with.overflow.i16",
                OverflowOperation {
                    operator: OverflowOperator::Subtract,
                    bits: IntegerBits::Bits32,
                    sign: IntegerSign::Signed,
                } => "llvm.ssub.with.overflow.i32",
                OverflowOperation {
                    operator: OverflowOperator::Subtract,
                    bits: IntegerBits::Bits64 | IntegerBits::Normal,
                    sign: IntegerSign::Signed,
                } => "llvm.ssub.with.overflow.i64",
                OverflowOperation {
                    operator: OverflowOperator::Subtract,
                    bits: IntegerBits::Bits8,
                    sign: IntegerSign::Unsigned,
                } => "llvm.usub.with.overflow.i8",
                OverflowOperation {
                    operator: OverflowOperator::Subtract,
                    bits: IntegerBits::Bits16,
                    sign: IntegerSign::Unsigned,
                } => "llvm.usub.with.overflow.i16",
                OverflowOperation {
                    operator: OverflowOperator::Subtract,
                    bits: IntegerBits::Bits32,
                    sign: IntegerSign::Unsigned,
                } => "llvm.usub.with.overflow.i32",
                OverflowOperation {
                    operator: OverflowOperator::Subtract,
                    bits: IntegerBits::Bits64 | IntegerBits::Normal,
                    sign: IntegerSign::Unsigned,
                } => "llvm.usub.with.overflow.i64",
                OverflowOperation {
                    operator: OverflowOperator::Multiply,
                    bits: IntegerBits::Bits8,
                    sign: IntegerSign::Signed,
                } => "llvm.smul.with.overflow.i8",
                OverflowOperation {
                    operator: OverflowOperator::Multiply,
                    bits: IntegerBits::Bits16,
                    sign: IntegerSign::Signed,
                } => "llvm.smul.with.overflow.i16",
                OverflowOperation {
                    operator: OverflowOperator::Multiply,
                    bits: IntegerBits::Bits32,
                    sign: IntegerSign::Signed,
                } => "llvm.smul.with.overflow.i32",
                OverflowOperation {
                    operator: OverflowOperator::Multiply,
                    bits: IntegerBits::Bits64 | IntegerBits::Normal,
                    sign: IntegerSign::Signed,
                } => "llvm.smul.with.overflow.i64",
                OverflowOperation {
                    operator: OverflowOperator::Multiply,
                    bits: IntegerBits::Bits8,
                    sign: IntegerSign::Unsigned,
                } => "llvm.umul.with.overflow.i8",
                OverflowOperation {
                    operator: OverflowOperator::Multiply,
                    bits: IntegerBits::Bits16,
                    sign: IntegerSign::Unsigned,
                } => "llvm.umul.with.overflow.i16",
                OverflowOperation {
                    operator: OverflowOperator::Multiply,
                    bits: IntegerBits::Bits32,
                    sign: IntegerSign::Unsigned,
                } => "llvm.umul.with.overflow.i32",
                OverflowOperation {
                    operator: OverflowOperator::Multiply,
                    bits: IntegerBits::Bits64 | IntegerBits::Normal,
                    sign: IntegerSign::Unsigned,
                } => "llvm.umul.with.overflow.i64",
            };

            self.create_intrinsic_with_overflow(backend_type, &CString::new(name).unwrap())
        })
    }

    pub unsafe fn expect_i1(&self) -> (LLVMValueRef, LLVMTypeRef) {
        *self.expect_i1.get_or_init(|| {
            let mut parameter_types = [LLVMInt1Type(), LLVMInt1Type()];
            let is_var_arg = false;

            let signature = LLVMFunctionType(
                LLVMInt1Type(),
                parameter_types.as_mut_ptr(),
                parameter_types.len() as c_uint,
                is_var_arg as LLVMBool,
            );

            (
                LLVMAddFunction(self.module, cstr!("llvm.expect.i1").as_ptr(), signature),
                signature,
            )
        })
    }

    pub unsafe fn on_overflow(&self) -> (LLVMValueRef, LLVMTypeRef) {
        *self.on_overflow.get_or_init(|| {
            let mut parameter_types = [];
            let is_var_arg = false;

            let signature = LLVMFunctionType(
                LLVMInt1Type(),
                parameter_types.as_mut_ptr(),
                parameter_types.len() as c_uint,
                is_var_arg as LLVMBool,
            );

            let fn_value = LLVMAddFunction(
                self.module,
                cstr!("$__adept_overflow_panic__").as_ptr(),
                signature,
            );
            let basicblock = LLVMAppendBasicBlock(fn_value, cstr!("").as_ptr());

            let builder = Builder::new();
            LLVMPositionBuilderAtEnd(builder.get(), basicblock);

            {
                let (printf_fn, printf_fn_type) = self.printf();
                let mut args = [build_literal_cstring(
                    self.module,
                    &CString::new("panic: integer overflow\n").unwrap(),
                )];
                LLVMBuildCall2(
                    builder.get(),
                    printf_fn_type,
                    printf_fn,
                    args.as_mut_ptr(),
                    args.len().try_into().unwrap(),
                    cstr!("").as_ptr(),
                );

                let (abort_fn, abort_fn_type) = self.abort();
                let mut args = [];
                LLVMBuildCall2(
                    builder.get(),
                    abort_fn_type,
                    abort_fn,
                    args.as_mut_ptr(),
                    args.len().try_into().unwrap(),
                    cstr!("").as_ptr(),
                );
            }

            LLVMBuildUnreachable(builder.get());

            (fn_value, signature)
        })
    }

    pub unsafe fn printf(&self) -> (LLVMValueRef, LLVMTypeRef) {
        *self.printf.get_or_init(|| {
            let existing = LLVMGetNamedFunction(self.module, cstr!("printf").as_ptr());
            let mut parameter_types = [LLVMPointerType(LLVMInt8Type(), 0)];
            let is_var_arg = true;

            let signature = LLVMFunctionType(
                LLVMInt32Type(),
                parameter_types.as_mut_ptr(),
                parameter_types.len() as c_uint,
                is_var_arg as LLVMBool,
            );

            let fn_value = if existing.is_null() {
                LLVMAddFunction(self.module, cstr!("printf").as_ptr(), signature)
            } else {
                existing
            };

            (fn_value, signature)
        })
    }

    pub unsafe fn abort(&self) -> (LLVMValueRef, LLVMTypeRef) {
        *self.abort.get_or_init(|| {
            let existing = LLVMGetNamedFunction(self.module, cstr!("abort").as_ptr());
            let mut parameter_types = [];
            let is_var_arg = false;

            let signature = LLVMFunctionType(
                LLVMVoidType(),
                parameter_types.as_mut_ptr(),
                parameter_types.len() as c_uint,
                is_var_arg as LLVMBool,
            );

            let fn_value = if existing.is_null() {
                LLVMAddFunction(self.module, cstr!("abort").as_ptr(), signature)
            } else {
                existing
            };

            (fn_value, signature)
        })
    }

    unsafe fn create_intrinsic_with_overflow(
        &self,
        signed_integer_type: LLVMTypeRef,
        intrinsic_name: &CStr,
    ) -> (LLVMValueRef, LLVMTypeRef) {
        let mut parameter_types = [signed_integer_type, signed_integer_type];
        let mut return_elements = [signed_integer_type, LLVMInt1Type()];
        let return_type = LLVMStructType(return_elements.as_mut_ptr(), 2, false.into());
        let is_var_arg = false;

        let signature = LLVMFunctionType(
            return_type,
            parameter_types.as_mut_ptr(),
            parameter_types.len() as c_uint,
            is_var_arg as LLVMBool,
        );

        (
            LLVMAddFunction(self.module, intrinsic_name.as_ptr(), signature),
            signature,
        )
    }
}
