mod direct;
mod extend;
mod indirect;
mod kinds;
mod offset_align;
mod show_llvm_type;

pub use self::{direct::DirectOptions, extend::ExtendOptions, indirect::IndirectOptions};
use self::{
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
    pub fn new_direct(options: DirectOptions) -> Self {
        Self {
            kind: ABITypeKind::Direct(Direct {
                offset_align: OffsetAlign {
                    offset: options.offset,
                    align: options.align_bytes,
                },
                coerce_to_type: options.coerce_to_type,
                padding: options.padding,
                in_register: false,
                can_be_flattened: options.can_be_flattened,
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

        let is_unpadded_struct = is_struct_type(unpadded_coerce_to_type);

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

    pub fn coerce_to_type(&self) -> Option<Option<LLVMTypeRef>> {
        match &self.kind {
            ABITypeKind::Direct(direct) => Some(direct.coerce_to_type),
            ABITypeKind::Extend(extend) => Some(extend.coerce_to_type),
            ABITypeKind::CoerceAndExpand(coerce_and_expand) => {
                Some(Some(coerce_and_expand.coerce_to_type))
            }
            _ => None,
        }
    }

    pub fn get_direct_offset_align(&self) -> Option<OffsetAlign> {
        match &self.kind {
            ABITypeKind::Direct(direct) => Some(direct.offset_align),
            ABITypeKind::Extend(extend) => Some(extend.offset_align),
            _ => None,
        }
    }

    pub fn get_direct_offset(&self) -> Option<u32> {
        self.get_direct_offset_align().map(|info| info.offset)
    }

    pub fn get_direct_align(&self) -> Option<u32> {
        self.get_direct_offset_align().map(|info| info.align)
    }

    pub fn is_sign_extend(&self) -> bool {
        match &self.kind {
            ABITypeKind::Extend(extend) => extend.signext,
            _ => false,
        }
    }

    pub fn padding_type(&self) -> Option<Option<LLVMTypeRef>> {
        match &self.kind {
            ABITypeKind::Direct(direct) => Some(direct.padding),
            ABITypeKind::Extend(extend) => Some(extend.padding),
            ABITypeKind::Indirect(indirect) => Some(indirect.padding),
            ABITypeKind::IndirectAliased(indirect_aliased) => Some(indirect_aliased.padding),
            ABITypeKind::Expand(expand) => Some(expand.padding),
            _ => None,
        }
    }

    pub fn padding_in_register(&self) -> bool {
        self.padding_in_register
    }

    pub fn coerce_and_expand_type(&self) -> Option<LLVMTypeRef> {
        match &self.kind {
            ABITypeKind::CoerceAndExpand(coerce_and_expand) => {
                Some(coerce_and_expand.coerce_to_type)
            }
            _ => None,
        }
    }

    pub fn unpadded_coerce_and_expand_type(&self) -> Option<LLVMTypeRef> {
        match &self.kind {
            ABITypeKind::CoerceAndExpand(coerce_and_expand) => {
                Some(coerce_and_expand.unpadded_coerce_and_expand_type)
            }
            _ => None,
        }
    }

    pub fn coerce_and_expand_type_sequence(&self) -> Vec<LLVMTypeRef> {
        match &self.kind {
            ABITypeKind::CoerceAndExpand(coerce_and_expand) => {
                let unpadded = coerce_and_expand.unpadded_coerce_and_expand_type;

                if is_struct_type(unpadded) {
                    get_struct_field_types(unpadded)
                } else {
                    vec![unpadded]
                }
            }
            _ => panic!("invalid call to coerce_and_expand_type_sequence"),
        }
    }

    pub fn in_register(&self) -> Option<bool> {
        match &self.kind {
            ABITypeKind::Direct(direct) => Some(direct.in_register),
            ABITypeKind::Extend(extend) => Some(extend.in_register),
            ABITypeKind::Indirect(indirect) => Some(indirect.in_register),
            _ => None,
        }
    }

    pub fn indirect_align(&self) -> Option<ByteCount> {
        match &self.kind {
            ABITypeKind::Indirect(indirect) => Some(indirect.align),
            ABITypeKind::IndirectAliased(indirect_aliased) => Some(indirect_aliased.align),
            _ => None,
        }
    }

    pub fn indirect_byval(&self) -> Option<bool> {
        match &self.kind {
            ABITypeKind::Indirect(indirect) => Some(indirect.byval),
            _ => None,
        }
    }

    pub fn indirect_address_space(&self) -> Option<u32> {
        match &self.kind {
            ABITypeKind::IndirectAliased(indirect_aliased) => Some(indirect_aliased.address_space),
            _ => None,
        }
    }

    pub fn indirect_realign(&self) -> Option<bool> {
        match &self.kind {
            ABITypeKind::Indirect(indirect) => Some(indirect.realign),
            ABITypeKind::IndirectAliased(indirect_aliased) => Some(indirect_aliased.realign),
            _ => None,
        }
    }

    pub fn is_sret_after_this(&self) -> Option<bool> {
        match &self.kind {
            ABITypeKind::Indirect(indirect) => Some(indirect.sret_after_this),
            _ => None,
        }
    }

    pub fn alloca_field_index(&self) -> Option<u32> {
        match &self.kind {
            ABITypeKind::InAlloca(in_alloca) => Some(in_alloca.alloca_field_index),
            _ => None,
        }
    }

    pub fn in_alloca_indirect(&self) -> Option<bool> {
        match &self.kind {
            ABITypeKind::InAlloca(in_alloca) => Some(in_alloca.indirect),
            _ => None,
        }
    }

    pub fn in_alloca_sret(&self) -> Option<bool> {
        match &self.kind {
            ABITypeKind::InAlloca(in_alloca) => Some(in_alloca.sret),
            _ => None,
        }
    }

    pub fn can_be_flattened(&self) -> Option<bool> {
        match &self.kind {
            ABITypeKind::Direct(direct) => Some(direct.can_be_flattened),
            _ => None,
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

fn is_struct_type(ty: LLVMTypeRef) -> bool {
    return unsafe { LLVMGetTypeKind(ty) } == LLVMTypeKind::LLVMStructTypeKind;
}