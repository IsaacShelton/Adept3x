use super::{offset_align::OffsetAlign, show_llvm_type::ShowLLVMType};
use crate::{
    build_llvm_ir::{
        backend_type::to_backend_type, ctx::BackendCtx, llvm_type_ref_ext::LLVMTypeRefExt,
    },
    ir,
    target_layout::TypeLayoutCache,
};
use core::fmt::Debug;
use data_units::ByteUnits;
use diagnostics::ErrorDiagnostic;
use itertools::Itertools;
use llvm_sys::{
    LLVMTypeKind,
    core::{LLVMCountStructElementTypes, LLVMGetTypeKind},
    prelude::LLVMTypeRef,
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

impl Indirect {
    pub fn sret_position(&self) -> u8 {
        if self.sret_after_this { 1 } else { 0 }
    }
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
    pub fn expand<'env>(
        ctx: &BackendCtx<'_, 'env>,
        ir_type: &'env ir::Type<'env>,
    ) -> Result<Vec<LLVMTypeRef>, ErrorDiagnostic> {
        let expansion = get_type_expansion(ctx, ir_type, &ctx.type_layout_cache, ctx.ir_module);

        match expansion {
            TypeExpansion::FixedArray(fixed_array) => {
                let expanded_element = Self::expand(ctx, &fixed_array.inner)?;

                Ok(expanded_element
                    .iter()
                    .copied()
                    .cycle()
                    .take(expanded_element.len() * usize::try_from(fixed_array.length).unwrap())
                    .collect())
            }
            TypeExpansion::Record(fields) => fields
                .iter()
                .map(|field| Self::expand(ctx, &field.ir_type))
                .fold_ok(vec![], |mut acc, expanded| {
                    acc.extend(expanded);
                    acc
                }),
            TypeExpansion::Complex(inner) => {
                let expanded_element = Self::expand(ctx, inner)?;

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
pub enum TypeExpansion<'env> {
    FixedArray(&'env ir::FixedArray<'env>),
    Record(&'env [ir::Field<'env>]),
    Complex(&'env ir::Type<'env>),
    None,
}

impl Expand {
    pub fn expanded_size<'env>(
        &self,
        ctx: &BackendCtx<'_, 'env>,
        arg_type: &'env ir::Type<'env>,
        type_layout_cache: &TypeLayoutCache<'env>,
        ir_module: &'env ir::Ir<'env>,
    ) -> usize {
        get_type_expansion_size(ctx, arg_type, type_layout_cache, ir_module)
    }
}

fn get_type_expansion_size<'env>(
    ctx: &BackendCtx<'_, 'env>,
    ir_type: &'env ir::Type<'env>,
    type_layout_cache: &TypeLayoutCache<'env>,
    ir_module: &'env ir::Ir<'env>,
) -> usize {
    let expansion = get_type_expansion(ctx, ir_type, type_layout_cache, ir_module);

    match expansion {
        TypeExpansion::FixedArray(fixed_array) => {
            usize::try_from(fixed_array.length).unwrap()
                * get_type_expansion_size(ctx, &fixed_array.inner, type_layout_cache, ir_module)
        }
        TypeExpansion::Record(fields) => fields
            .iter()
            .map(|field| get_type_expansion_size(ctx, &field.ir_type, type_layout_cache, ir_module))
            .sum(),
        TypeExpansion::Complex(..) => 2,
        TypeExpansion::None => 1,
    }
}

pub fn get_type_expansion<'env>(
    ctx: &BackendCtx<'_, 'env>,
    ir_type: &'env ir::Type<'env>,
    _type_layout_cache: &TypeLayoutCache<'env>,
    ir_module: &'env ir::Ir<'env>,
) -> TypeExpansion<'env> {
    if let ir::Type::FixedArray(fixed_array) = ir_type {
        assert!(
            !fixed_array.inner.is_vector(),
            "expanding vector types is not supported yet, see mention in ParamValues::push_expand"
        );

        return TypeExpansion::FixedArray(fixed_array);
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

                    assert!(
                        !field.ir_type.is_vector(),
                        "expanding vector types is not supported yet, see mention in ParamValues::push_expand"
                    );
                    true
                }
            })
            .cloned()
            .collect_vec();

        return TypeExpansion::Record(ctx.alloc.alloc_slice_fill_iter(fields.into_iter()));
    }

    if let ir::Type::Complex(complex) = ir_type {
        return TypeExpansion::Complex(complex.element_type);
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
    pub alignment: ByteUnits,
}

impl CoerceAndExpand {
    pub fn expanded_type_sequence(&self) -> Vec<LLVMTypeRef> {
        let is_struct = unsafe {
            LLVMGetTypeKind(self.unpadded_coerce_and_expand_type)
                == LLVMTypeKind::LLVMStructTypeKind
        };

        if is_struct {
            self.unpadded_coerce_and_expand_type.field_types()
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
