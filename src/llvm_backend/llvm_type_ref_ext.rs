use crate::data_units::BitUnits;
use llvm_sys::{
    core::{
        LLVMArrayType2, LLVMCountStructElementTypes, LLVMGetElementType, LLVMGetIntTypeWidth,
        LLVMGetStructElementTypes, LLVMGetTypeKind, LLVMIntType,
    },
    prelude::LLVMTypeRef,
    LLVMType, LLVMTypeKind,
};
use std::ptr::null_mut;

pub trait LLVMTypeRefExt: Sized + Copy {
    fn new_int(bits: impl Into<BitUnits>) -> Self;
    fn new_array(inner: Self, length: u64) -> Self;

    fn is_pointer(self) -> bool;
    fn is_integer(self) -> bool;
    fn is_float(self) -> bool;
    fn is_double(self) -> bool;
    fn is_floating_point(self) -> bool;
    fn integer_width(self) -> BitUnits;
    fn is_struct(self) -> bool;
    fn is_array(self) -> bool;
    fn is_vector(self) -> bool;
    fn num_fields(self) -> usize;
    fn field_types(self) -> Vec<LLVMTypeRef>;
    fn element_type(self) -> LLVMTypeRef;

    fn is_integer_or_pointer(self) -> bool {
        self.is_integer() || self.is_pointer()
    }

    fn is_i8(self) -> bool {
        self.is_integer() && self.integer_width() == BitUnits::of(8)
    }

    fn is_i16(self) -> bool {
        self.is_integer() && self.integer_width() == BitUnits::of(16)
    }

    fn is_i32(self) -> bool {
        self.is_integer() && self.integer_width() == BitUnits::of(32)
    }

    fn is_i64(self) -> bool {
        self.is_integer() && self.integer_width() == BitUnits::of(64)
    }

    fn is_i128(self) -> bool {
        // NOTE: We don't support 128-bit integers yet
        false
    }
}

impl LLVMTypeRefExt for LLVMTypeRef {
    fn new_int(bits: impl Into<BitUnits>) -> Self {
        unsafe { LLVMIntType(bits.into().bits().try_into().unwrap()) }
    }

    fn new_array(inner: Self, length: u64) -> Self {
        unsafe { LLVMArrayType2(inner, length) }
    }

    fn is_pointer(self) -> bool {
        unsafe { LLVMGetTypeKind(self) == LLVMTypeKind::LLVMPointerTypeKind }
    }

    fn is_integer(self) -> bool {
        unsafe { LLVMGetTypeKind(self) == LLVMTypeKind::LLVMIntegerTypeKind }
    }

    fn is_float(self) -> bool {
        unsafe { LLVMGetTypeKind(self) == LLVMTypeKind::LLVMFloatTypeKind }
    }

    fn is_double(self) -> bool {
        unsafe { LLVMGetTypeKind(self) == LLVMTypeKind::LLVMDoubleTypeKind }
    }

    fn is_floating_point(self) -> bool {
        self.is_float() || self.is_double()
    }

    fn integer_width(self) -> BitUnits {
        BitUnits::of(unsafe { LLVMGetIntTypeWidth(self) }.into())
    }

    fn is_struct(self) -> bool {
        unsafe { LLVMGetTypeKind(self) == LLVMTypeKind::LLVMStructTypeKind }
    }

    fn is_array(self) -> bool {
        unsafe { LLVMGetTypeKind(self) == LLVMTypeKind::LLVMArrayTypeKind }
    }

    fn is_vector(self) -> bool {
        unsafe { LLVMGetTypeKind(self) == LLVMTypeKind::LLVMVectorTypeKind }
    }

    fn num_fields(self) -> usize {
        assert!(self.is_struct());
        unsafe { LLVMCountStructElementTypes(self) as usize }
    }

    fn field_types(self) -> Vec<LLVMTypeRef> {
        assert!(self.is_struct());

        let mut elements = vec![null_mut::<LLVMType>(); self.num_fields()];

        unsafe {
            LLVMGetStructElementTypes(self, elements.as_mut_ptr());
        }

        elements
    }

    fn element_type(self) -> LLVMTypeRef {
        unsafe { LLVMGetElementType(self) }
    }
}
