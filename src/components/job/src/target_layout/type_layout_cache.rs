use crate::{
    ir,
    target_layout::{
        TargetLayout,
        record_layout::{ItaniumRecordLayoutBuilder, RecordInfo},
        type_layout::{AlignRequirement, TypeLayout},
    },
};
use arena::LockFreeArena;
use data_units::ByteUnits;
use diagnostics::Diagnostics;
use once_map::sync::OnceMap;
use target::Target;

#[derive(Debug)]
pub struct TypeLayoutCache<'env> {
    memo: OnceMap<&'env ir::Type<'env>, TypeLayout>,
    pub target: Target,
    pub structs: &'env LockFreeArena<ir::StructId, ir::Struct<'env>>,
    pub diagnostics: &'env Diagnostics<'env>,
}

impl<'env> TypeLayoutCache<'env> {
    pub fn new(
        target: Target,
        structs: &'env LockFreeArena<ir::StructId, ir::Struct<'env>>,
        diagnostics: &'env Diagnostics<'env>,
    ) -> Self {
        Self {
            memo: OnceMap::new(),
            target,
            structs,
            diagnostics,
        }
    }

    pub fn get(&self, ir_type: &'env ir::Type<'env>) -> TypeLayout {
        self.memo
            .map_insert_ref(&ir_type, |ty| *ty, |key| self.get_impl(key), |_k, v| *v)
    }

    fn get_impl(&self, ir_type: &'env ir::Type<'env>) -> TypeLayout {
        match ir_type {
            ir::Type::Ptr(_) | ir::Type::FuncPtr => self.target.pointer_layout(),
            ir::Type::Bool => self.target.bool_layout(),
            ir::Type::S8 | ir::Type::U8 => TypeLayout::basic(ByteUnits::of(1)),
            ir::Type::S16 | ir::Type::U16 => TypeLayout::basic(ByteUnits::of(2)),
            ir::Type::S32 | ir::Type::U32 => TypeLayout::basic(ByteUnits::of(4)),
            ir::Type::S64 | ir::Type::U64 => TypeLayout::basic(ByteUnits::of(8)),
            ir::Type::F32 => TypeLayout::basic(ByteUnits::of(4)),
            ir::Type::F64 => TypeLayout::basic(ByteUnits::of(8)),
            ir::Type::Void => TypeLayout {
                width: ByteUnits::of(0),
                alignment: ByteUnits::of(0),
                unadjusted_alignment: ByteUnits::of(1),
                align_requirement: AlignRequirement::None,
            },
            ir::Type::Union(_) => todo!("get_impl for ir::Type::Union"),
            ir::Type::Struct(struct_ref) => {
                let structure = &self.structs[*struct_ref];

                let info = RecordInfo::from_struct(structure);
                self.get_impl_record_layout(&info, Some(structure.name))
            }
            ir::Type::AnonymousComposite(type_composite) => {
                let info = RecordInfo::from_composite(type_composite);
                self.get_impl_record_layout(&info, None)
            }
            ir::Type::FixedArray(fixed_array) => {
                let element_info = self.get(&fixed_array.inner);

                TypeLayout {
                    width: element_info.width * fixed_array.length,
                    alignment: element_info.alignment,
                    unadjusted_alignment: element_info.alignment,
                    align_requirement: element_info.align_requirement,
                }
            }
            ir::Type::Vector(_) => todo!("TypeLayoutCache::get_impl for ir::Type::Vector"),
            ir::Type::Complex(_) => todo!("TypeLayoutCache::get_impl for ir::Type::Complex"),
            ir::Type::Atomic(_) => todo!("TypeLayoutCache::get_impl for ir::Type::Atomic"),
            ir::Type::IncompleteArray(_) => {
                todo!("TypeLayoutCache::get_impl for ir::Type::IncompleteArray")
            }
        }
    }

    fn get_impl_record_layout(&self, info: &RecordInfo<'env>, name: Option<&str>) -> TypeLayout {
        // TODO: We should cache this

        let record_layout =
            ItaniumRecordLayoutBuilder::generate(self, self.diagnostics, info, name);

        // NOTE: We don't support alignment attributes yet,
        // so this will always be none
        let alignment_requirement = AlignRequirement::None;

        TypeLayout {
            width: record_layout.size,
            alignment: record_layout.alignment,
            align_requirement: alignment_requirement,
            unadjusted_alignment: record_layout.unadjusted_alignment,
        }
    }
}
