use super::{record_layout::record_info::RecordInfo, TargetInfo};
use crate::{
    data_units::{BitUnits, ByteUnits},
    ir,
    target_info::record_layout::{itanium::ItaniumRecordLayoutBuilder, record_info},
};
use once_map::unsync::OnceMap;

#[derive(Copy, Clone, Debug, Default)]
pub enum AlignmentRequirement {
    #[default]
    None,
    RequiredByTypedefAttribute,
    RequiredByRecordAttribute,
}

#[derive(Copy, Clone, Debug)]
pub struct TypeInfo {
    pub width: ByteUnits,
    pub alignment: ByteUnits,
    pub unadjusted_alignment: ByteUnits,
    pub alignment_requirement: AlignmentRequirement,
}

impl TypeInfo {
    pub fn basic(size: ByteUnits) -> Self {
        Self {
            width: size.into(),
            alignment: size,
            unadjusted_alignment: size,
            alignment_requirement: AlignmentRequirement::None,
        }
    }
}

#[derive(Debug)]
pub struct TypeInfoManager<'a> {
    memo: OnceMap<ir::Type, TypeInfo>,
    structures: &'a ir::Structures,
}

impl<'a> TypeInfoManager<'a> {
    pub fn new(structures: &'a ir::Structures) -> Self {
        Self {
            memo: OnceMap::new(),
            structures,
        }
    }

    pub fn get_type_info(&self, ir_type: &ir::Type, target_info: &TargetInfo) -> TypeInfo {
        self.memo.map_insert_ref(
            ir_type,
            |ty| ty.clone(),
            |key| self.get_type_info_impl(key, target_info),
            |_k, v| *v,
        )
    }

    fn get_type_info_impl(&self, ir_type: &ir::Type, target_info: &TargetInfo) -> TypeInfo {
        match ir_type {
            ir::Type::Pointer(_) | ir::Type::FunctionPointer => target_info.pointer_layout(),
            ir::Type::Boolean => target_info.bool_layout(),
            ir::Type::S8 | ir::Type::U8 => TypeInfo::basic(ByteUnits::of(1)),
            ir::Type::S16 | ir::Type::U16 => TypeInfo::basic(ByteUnits::of(2)),
            ir::Type::S32 | ir::Type::U32 => TypeInfo::basic(ByteUnits::of(4)),
            ir::Type::S64 | ir::Type::U64 => TypeInfo::basic(ByteUnits::of(8)),
            ir::Type::F32 => TypeInfo::basic(ByteUnits::of(4)),
            ir::Type::F64 => TypeInfo::basic(ByteUnits::of(8)),
            ir::Type::Void => TypeInfo {
                width: ByteUnits::of(0),
                alignment: ByteUnits::of(0),
                unadjusted_alignment: ByteUnits::of(1),
                alignment_requirement: AlignmentRequirement::None,
            },
            ir::Type::Structure(structure_ref) => {
                let structure = self
                    .structures
                    .get(structure_ref)
                    .expect("referenced structure to exist");

                let info = record_info::info_from_structure(structure);
                type_info_for_record_layout(self, target_info, &info)
            }
            ir::Type::AnonymousComposite(type_composite) => {
                let info = record_info::info_from_composite(type_composite);
                type_info_for_record_layout(self, target_info, &info)
            }
            ir::Type::FixedArray(fixed_array) => {
                let element_info = self.get_type_info(&fixed_array.inner, target_info);

                TypeInfo {
                    width: element_info.width * fixed_array.size,
                    alignment: element_info.alignment,
                    unadjusted_alignment: element_info.alignment,
                    alignment_requirement: element_info.alignment_requirement,
                }
            }
            ir::Type::Vector(_) => todo!("get_type_info_impl for ir::Type::Vector"),
            ir::Type::Complex(_) => todo!("get_type_info_impl for ir::Type::Complex"),
            ir::Type::Atomic(_) => todo!("get_type_info_impl for ir::Type::Atomic"),
            ir::Type::IncompleteArray(_) => {
                todo!("get_type_info_impl for ir::Type::IncompleteArray")
            }
        }
    }

    fn get_ast_record_layout<'t>(
        &self,
        target_info: &TargetInfo,
        info: &RecordInfo<'t>,
    ) -> ASTRecordLayout {
        // TODO: We need to cache this

        let mut builder = ItaniumRecordLayoutBuilder::new(self, target_info, None);
        builder.layout(info);

        let size = ByteUnits::try_from(builder.size).unwrap();

        ASTRecordLayout {
            size,
            alignment: builder.alignment,
            preferred_alignment: builder.preferred_alignment,
            unadjusted_alignment: builder.unadjusted_alignment,
            required_alignment: builder.alignment,
            data_size: size,
            field_offsets: builder.field_offsets,
        }
    }
}

#[derive(Debug)]
pub struct ASTRecordLayout {
    pub size: ByteUnits,
    pub alignment: ByteUnits,
    pub preferred_alignment: ByteUnits,
    pub unadjusted_alignment: ByteUnits,
    pub required_alignment: ByteUnits,
    pub data_size: ByteUnits,
    pub field_offsets: Vec<BitUnits>,
}

fn type_info_for_record_layout<'t>(
    type_info_manager: &TypeInfoManager,
    target_info: &TargetInfo,
    info: &RecordInfo<'t>,
) -> TypeInfo {
    let record_layout = type_info_manager.get_ast_record_layout(target_info, info);

    // NOTE: We don't support alignment attributes yet,
    // so this will always be none
    let alignment_requirement = AlignmentRequirement::None;

    TypeInfo {
        width: record_layout.size,
        alignment: record_layout.alignment,
        alignment_requirement,
        unadjusted_alignment: record_layout.unadjusted_alignment,
    }
}
