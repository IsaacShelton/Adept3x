use super::TargetInfo;
use crate::{ir, target_info::record_layout::record_info};
use once_map::unsync::OnceMap;

#[derive(Copy, Clone, Debug)]
pub struct TypeInfo {
    pub width_bytes: u64,
    pub align_bytes: u32,
    pub unadjusted_align_bytes: u32,
}

impl TypeInfo {
    pub fn basic(bytes: u32) -> Self {
        Self {
            width_bytes: bytes.into(),
            align_bytes: bytes,
            unadjusted_align_bytes: bytes,
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
            ir::Type::S8 | ir::Type::U8 => TypeInfo::basic(1),
            ir::Type::S16 | ir::Type::U16 => TypeInfo::basic(2),
            ir::Type::S32 | ir::Type::U32 => TypeInfo::basic(4),
            ir::Type::S64 | ir::Type::U64 => TypeInfo::basic(8),
            ir::Type::F32 => TypeInfo::basic(4),
            ir::Type::F64 => TypeInfo::basic(8),
            ir::Type::Void => TypeInfo {
                width_bytes: 0,
                align_bytes: 1,
                unadjusted_align_bytes: 1,
            },
            ir::Type::Structure(structure_ref) => {
                let structure = self
                    .structures
                    .get(structure_ref)
                    .expect("referenced structure to exist");

                let _info = record_info::info_from_structure(structure);

                /*
                let record_layout = RecordLayout::new(structure_ref);
                Ok(record_layout.type_info)

                let record_layout = get_record_layout();

                let width_bytes = 0;
                let align_bytes = 0;
                let align_requirement = if false { Some(RequiredByRecord) } else { None };
                */

                todo!("get_type_info_impl for ir::Type::Structure")
            }
            ir::Type::AnonymousComposite(type_composite) => {
                let _info = record_info::info_from_composite(type_composite);

                /*
                let record_layout = RecordLayout::new(structure_ref);
                Ok(record_layout.type_info)
                */
                todo!("get_type_info_impl for ir::Type::AnonymousComposite")
            }
            ir::Type::FixedArray(fixed_array) => {
                let element_info = self.get_type_info(&fixed_array.inner, target_info);

                TypeInfo {
                    width_bytes: fixed_array.size * element_info.width_bytes,
                    align_bytes: element_info.align_bytes,
                    unadjusted_align_bytes: element_info.align_bytes,
                }
            }
            ir::Type::Vector(_) => todo!("get_type_info_impl for ir::Type::Vector"),
            ir::Type::Complex(_) => todo!("get_type_info_impl for ir::Type::Complex"),
            ir::Type::Atomic(_) => todo!("get_type_info_impl for ir::Type::Atomic"),
        }
    }
}