use super::{get_struct_field_types, offset_align::OffsetAlign, show_llvm_type::ShowLLVMType};
use crate::{
    data_units::ByteUnits,
    ir,
    llvm_backend::{backend_type::to_backend_type, ctx::BackendCtx, error::BackendError},
    target_info::type_layout::TypeLayoutCache,
};
use core::fmt::Debug;
use itertools::Itertools;
use llvm_sys::{
    core::{LLVMCountStructElementTypes, LLVMGetTypeKind},
    prelude::LLVMTypeRef,
    LLVMTypeKind,
};

#[derive(Clone)]
pub struct Direct {
    pub offset_align: OffsetAlign,
    pub coerce_to_type: Option<LLVMTypeRef>,
    pub padding: Option<LLVMTypeRef>,
    pub in_register: bool,
    pub can_be_flattened: bool,
}

impl Debug for Direct {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Direct")
            .field("offset_align", &self.offset_align)
            .field("coerce_to_type", &self.coerce_to_type.map(ShowLLVMType))
            .field("padding", &self.padding.map(ShowLLVMType))
            .field("in_register", &self.in_register)
            .field("can_be_flattened", &self.can_be_flattened)
            .finish()
    }
}

#[derive(Clone)]
pub struct Extend {
    pub offset_align: OffsetAlign,
    pub coerce_to_type: Option<LLVMTypeRef>,
    pub padding: Option<LLVMTypeRef>,
    pub in_register: bool,
    pub signext: bool,
}

impl Debug for Extend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Extend")
            .field("offset_align", &self.offset_align)
            .field("coerce_to_type", &self.coerce_to_type.map(ShowLLVMType))
            .field("padding", &self.padding.map(ShowLLVMType))
            .field("in_register", &self.in_register)
            .field("signext", &self.signext)
            .finish()
    }
}

#[derive(Clone)]
pub struct Indirect {
    pub padding: Option<LLVMTypeRef>,
    pub align: ByteUnits,
    pub byval: bool,
    pub realign: bool,
    pub sret_after_this: bool,
    pub in_register: bool,
}

impl Debug for Indirect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Indirect")
            .field("padding", &self.padding.map(ShowLLVMType))
            .field("align", &self.align)
            .field("byval", &self.byval)
            .field("realign", &self.realign)
            .field("sret_after_this", &self.sret_after_this)
            .field("in_register", &self.in_register)
            .finish()
    }
}

#[derive(Clone)]
pub struct IndirectAliased {
    pub padding: Option<LLVMTypeRef>,
    pub align: ByteUnits,
    pub realign: bool,
    pub address_space: u32,
}

impl Debug for IndirectAliased {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IndirectAliased")
            .field("padding", &self.padding.map(ShowLLVMType))
            .field("align", &self.align)
            .field("realign", &self.realign)
            .field("address_space", &self.address_space)
            .finish()
    }
}

#[derive(Clone)]
pub struct Expand {
    pub padding: Option<LLVMTypeRef>,
}

impl Expand {
    pub fn expand(ctx: &BackendCtx, ir_type: &ir::Type) -> Result<Vec<LLVMTypeRef>, BackendError> {
        let expansion = get_type_expansion(ir_type, &ctx.type_layout_cache, ctx.ir_module);

        match expansion {
            TypeExpansion::FixedArray(fixed_array) => {
                let expanded_element = Self::expand(ctx, &fixed_array.inner)?;

                Ok(expanded_element
                    .iter()
                    .copied()
                    .cycle()
                    .take(expanded_element.len() * usize::try_from(fixed_array.size).unwrap())
                    .collect())
            }
            TypeExpansion::Record(fields) => fields
                .iter()
                .map(|field| Self::expand(ctx, &field.ir_type))
                .fold_ok(vec![], |mut acc, expanded| {
                    acc.extend(expanded.into_iter());
                    acc
                }),
            TypeExpansion::Complex(inner) => {
                let expanded_element = Self::expand(ctx, &inner)?;

                Ok(expanded_element
                    .iter()
                    .copied()
                    .cycle()
                    .take(expanded_element.len() * 2)
                    .collect())
            }
            TypeExpansion::None => Ok(vec![unsafe {
                to_backend_type(ctx.for_making_type(), ir_type)?
            }]),
        }
    }
}

#[derive(Clone, Debug)]
pub enum TypeExpansion {
    FixedArray(Box<ir::FixedArray>),
    Record(Vec<ir::Field>),
    Complex(ir::Type),
    None,
}

impl Expand {
    pub fn expanded_size(
        &self,
        arg_type: &ir::Type,
        type_layout_cache: &TypeLayoutCache,
        ir_module: &ir::Module,
    ) -> usize {
        get_type_expansion_size(arg_type, type_layout_cache, ir_module)
    }
}

fn get_type_expansion_size(
    ir_type: &ir::Type,
    type_layout_cache: &TypeLayoutCache,
    ir_module: &ir::Module,
) -> usize {
    let expansion = get_type_expansion(ir_type, type_layout_cache, ir_module);

    match expansion {
        TypeExpansion::FixedArray(fixed_array) => {
            usize::try_from(fixed_array.size).unwrap()
                * get_type_expansion_size(&fixed_array.inner, type_layout_cache, ir_module)
        }
        TypeExpansion::Record(fields) => fields
            .iter()
            .map(|field| get_type_expansion_size(&field.ir_type, type_layout_cache, ir_module))
            .sum(),
        TypeExpansion::Complex(..) => 2,
        TypeExpansion::None => 1,
    }
}

fn get_type_expansion(
    ir_type: &ir::Type,
    _type_layout_cache: &TypeLayoutCache,
    ir_module: &ir::Module,
) -> TypeExpansion {
    if let ir::Type::FixedArray(fixed_array) = ir_type {
        return TypeExpansion::FixedArray(fixed_array.clone());
    }

    if ir_type.is_union() {
        todo!("get_type_expansion for unions not implemented yet");
    }

    if let Some(fields) = ir_type.struct_fields(ir_module) {
        let fields = fields
            .iter()
            .filter(|field| {
                if field.is_zero_length_bitfield() {
                    false
                } else {
                    assert!(!field.is_bitfield(), "can't expand bitfields members");
                    true
                }
            })
            .cloned()
            .collect_vec();

        return TypeExpansion::Record(fields);
    }

    if let ir::Type::Complex(complex) = ir_type {
        return TypeExpansion::Complex(complex.element_type.clone());
    }

    TypeExpansion::None
}

impl Debug for Expand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Expand")
            .field("padding", &self.padding.map(ShowLLVMType))
            .finish()
    }
}

#[derive(Clone, Debug)]
pub struct InAlloca {
    pub alloca_field_index: u32,
    pub sret: bool,
    pub indirect: bool,
}

#[derive(Clone)]
pub struct CoerceAndExpand {
    pub coerce_to_type: LLVMTypeRef,
    pub unpadded_coerce_and_expand_type: LLVMTypeRef,
}

impl CoerceAndExpand {
    pub fn expanded_type_sequence(&self) -> Vec<LLVMTypeRef> {
        let is_struct = unsafe {
            LLVMGetTypeKind(self.unpadded_coerce_and_expand_type)
                == LLVMTypeKind::LLVMStructTypeKind
        };

        if is_struct {
            get_struct_field_types(self.unpadded_coerce_and_expand_type)
        } else {
            vec![self.unpadded_coerce_and_expand_type]
        }
    }

    pub fn expanded_type_sequence_len(&self) -> usize {
        if unsafe { LLVMGetTypeKind(self.coerce_to_type) } == LLVMTypeKind::LLVMStructTypeKind {
            unsafe { LLVMCountStructElementTypes(self.coerce_to_type) }
                .try_into()
                .unwrap()
        } else {
            1
        }
    }
}

impl Debug for CoerceAndExpand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CoerceAndExpand")
            .field("coerce_to_type", &ShowLLVMType(self.coerce_to_type))
            .field(
                "unpadded_coerce_and_expand_type",
                &ShowLLVMType(self.unpadded_coerce_and_expand_type),
            )
            .finish()
    }
}
