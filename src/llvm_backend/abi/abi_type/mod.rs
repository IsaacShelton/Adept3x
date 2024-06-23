mod extend;
mod indirect;
mod kinds;
mod offset_align;

use self::{
    extend::ExtendOptions,
    indirect::IndirectOptions,
    kinds::{CoerceAndExpand, Direct, Expand, Extend, InAlloca, Indirect, IndirectAliased},
    offset_align::{ByteCount, OffsetAlign},
};
use crate::ir;
use derive_more::{Deref, IsVariant};
use llvm_sys::{
    core::{
        LLVMCountStructElementTypes, LLVMGetArrayLength2, LLVMGetElementType,
        LLVMGetStructElementTypes, LLVMGetTypeKind, LLVMGetVectorSize, LLVMInt8Type,
    },
    prelude::LLVMTypeRef,
    LLVMType, LLVMTypeKind,
};
use std::ptr::null_mut;

#[derive(Clone, Debug, Deref)]
pub struct ABIType {
    #[deref]
    pub kind: ABITypeKind,
    pub padding_in_register: bool,
}

#[derive(Clone, Debug, IsVariant)]
pub enum ABITypeKind {
    Direct(Direct),
    Extend(Extend),
    Indirect(Indirect),
    IndirectAliased(IndirectAliased),
    Ignore,
    Expand(Expand),
    CoerceAndExpand(CoerceAndExpand),
    InAlloca(InAlloca),
}

impl ABIType {
    pub fn new_direct(
        coerce_to_type: Option<LLVMTypeRef>,
        offset: Option<u32>,
        padding: Option<LLVMTypeRef>,
        can_be_flattened: Option<bool>,
        align: Option<u32>,
    ) -> Self {
        Self {
            kind: ABITypeKind::Direct(Direct {
                offset_align: OffsetAlign {
                    offset: offset.unwrap_or(0),
                    align: align.unwrap_or(0),
                },
                coerce_to_type,
                padding,
                in_register: false,
                can_be_flattened: can_be_flattened.unwrap_or(true),
            }),
            padding_in_register: false,
        }
    }

    pub fn new_direct_in_register(coerce_to_type: Option<LLVMTypeRef>) -> Self {
        Self {
            kind: ABITypeKind::Direct(Direct {
                offset_align: OffsetAlign::default(),
                coerce_to_type,
                padding: None,
                in_register: true,
                can_be_flattened: true,
            }),
            padding_in_register: false,
        }
    }

    pub fn new_sign_extend(
        ir_type: &ir::Type,
        coerce_to_type: Option<LLVMTypeRef>,
        options: ExtendOptions,
    ) -> Self {
        assert!(ir_type.is_integer_like());

        Self {
            kind: ABITypeKind::Extend(Extend {
                offset_align: OffsetAlign::default(),
                coerce_to_type,
                padding: None,
                in_register: options.in_register,
                signext: true,
            }),
            padding_in_register: false,
        }
    }

    pub fn new_zero_extend(
        ir_type: &ir::Type,
        coerce_to_type: Option<LLVMTypeRef>,
        options: ExtendOptions,
    ) -> Self {
        assert!(ir_type.is_integer_like());

        Self {
            kind: ABITypeKind::Extend(Extend {
                offset_align: OffsetAlign::default(),
                coerce_to_type,
                padding: None,
                in_register: options.in_register,
                signext: false,
            }),
            padding_in_register: false,
        }
    }

    pub fn new_extend(
        ir_type: &ir::Type,
        coerce_to_type: Option<LLVMTypeRef>,
        options: ExtendOptions,
    ) -> Self {
        match ir_type.is_signed() {
            Some(true) => Self::new_sign_extend(ir_type, coerce_to_type, options),
            Some(false) => Self::new_zero_extend(ir_type, coerce_to_type, options),
            None => panic!("invalid type"),
        }
    }

    pub fn new_extend_in_register(ir_type: &ir::Type, coerce_to_type: Option<LLVMTypeRef>) -> Self {
        Self::new_extend(
            ir_type,
            coerce_to_type,
            ExtendOptions {
                in_register: true,
                ..Default::default()
            },
        )
    }

    pub fn new_ignore() -> Self {
        Self {
            kind: ABITypeKind::Ignore,
            padding_in_register: false,
        }
    }

    pub fn new_indirect(
        alignment: ByteCount,
        byval: Option<bool>,
        realign: Option<bool>,
        padding: Option<LLVMTypeRef>,
        options: IndirectOptions,
    ) -> Self {
        Self {
            kind: ABITypeKind::Indirect(Indirect {
                padding,
                align: alignment,
                byval: byval.unwrap_or(true),
                realign: realign.unwrap_or(false),
                sret_after_this: false,
                in_register: options.in_register,
            }),
            padding_in_register: false,
        }
    }

    pub fn new_indirect_aliased(
        alignment: ByteCount,
        address_space: u32,
        realign: Option<bool>,
        padding: Option<LLVMTypeRef>,
    ) -> Self {
        Self {
            kind: ABITypeKind::IndirectAliased(IndirectAliased {
                padding,
                align: alignment,
                realign: realign.unwrap_or(false),
                address_space,
            }),
            padding_in_register: false,
        }
    }

    pub fn new_indirect_in_register(
        alignment: ByteCount,
        byval: Option<bool>,
        realign: Option<bool>,
        padding: Option<LLVMTypeRef>,
    ) -> Self {
        Self::new_indirect(
            alignment,
            byval,
            realign,
            padding,
            IndirectOptions {
                in_register: true,
                ..Default::default()
            },
        )
    }

    pub fn new_in_alloc(field_index: u32, indirect: Option<bool>) -> Self {
        Self {
            kind: ABITypeKind::InAlloca(InAlloca {
                alloca_field_index: field_index,
                sret: false,
                indirect: indirect.unwrap_or(false),
            }),
            padding_in_register: false,
        }
    }

    pub fn new_expand() -> Self {
        Self {
            kind: ABITypeKind::Expand(Expand { padding: None }),
            padding_in_register: false,
        }
    }

    pub fn new_expand_with_padding(
        padding_in_register: bool,
        padding: Option<LLVMTypeRef>,
    ) -> Self {
        Self {
            kind: ABITypeKind::Expand(Expand { padding }),
            padding_in_register,
        }
    }

    pub fn new_coerce_and_expand(
        coerce_to_type: LLVMTypeRef,
        unpadded_coerce_to_type: LLVMTypeRef,
    ) -> Self {
        assert_eq!(
            unsafe { LLVMGetTypeKind(coerce_to_type) },
            LLVMTypeKind::LLVMStructTypeKind
        );

        let is_unpadded_struct =
            unsafe { LLVMGetTypeKind(unpadded_coerce_to_type) } == LLVMTypeKind::LLVMStructTypeKind;

        assert!(
            is_unpadded_struct
                || (unsafe { LLVMGetTypeKind(unpadded_coerce_to_type) }
                    == LLVMTypeKind::LLVMVectorTypeKind
                    && unsafe { LLVMGetVectorSize(unpadded_coerce_to_type) } != 1)
                || (unsafe { LLVMGetTypeKind(unpadded_coerce_to_type) }
                    == LLVMTypeKind::LLVMArrayTypeKind
                    && unsafe { LLVMGetArrayLength2(unpadded_coerce_to_type) } != 1)
        );

        let mut unpadded_index = 0;
        let field_types = get_struct_field_types(coerce_to_type);
        let unpadded_field_types =
            is_unpadded_struct.then(|| get_struct_field_types(unpadded_coerce_to_type));

        for element_type in field_types.iter() {
            if is_padding_for_coerce_expand(*element_type) {
                continue;
            }

            if let Some(unpadded_field_types) = &unpadded_field_types {
                assert_eq!(unpadded_field_types[unpadded_index], *element_type);
            } else {
                assert_eq!(unpadded_index, 0);
                assert_eq!(unpadded_coerce_to_type, *element_type);
            }

            unpadded_index += 1;
        }

        if let Some(unpadded_field_types) = unpadded_field_types {
            assert_eq!(unpadded_index, unpadded_field_types.len());
        } else {
            assert_eq!(unpadded_index, 1);
        }

        Self {
            kind: ABITypeKind::CoerceAndExpand(CoerceAndExpand {
                coerce_to_type,
                unpadded_coerce_and_expand_type: unpadded_coerce_to_type,
            }),
            padding_in_register: false,
        }
    }
}

fn is_padding_for_coerce_expand(ty: LLVMTypeRef) -> bool {
    if unsafe { LLVMGetTypeKind(ty) == LLVMTypeKind::LLVMArrayTypeKind } {
        assert_eq!(unsafe { LLVMGetElementType(ty) }, unsafe { LLVMInt8Type() });
        true
    } else {
        false
    }
}

fn get_struct_field_types(struct_type: LLVMTypeRef) -> Vec<LLVMTypeRef> {
    assert!(unsafe { LLVMGetTypeKind(struct_type) } == LLVMTypeKind::LLVMStructTypeKind);

    let num_elements = unsafe { LLVMCountStructElementTypes(struct_type) } as usize;
    let mut elements = vec![null_mut::<LLVMType>(); num_elements];

    unsafe {
        LLVMGetStructElementTypes(struct_type, elements.as_mut_ptr());
    }
    elements
}
