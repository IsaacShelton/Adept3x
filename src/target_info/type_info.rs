use super::TargetInfo;
use crate::ir;
use once_map::unsync::OnceMap;

#[derive(Copy, Clone, Debug)]
pub struct TypeInfo {
    pub width_bytes: u64,
    pub align_bytes: u32,
}

impl TypeInfo {
    pub fn basic(bytes: u32) -> Self {
        Self {
            width_bytes: bytes.into(),
            align_bytes: bytes,
        }
    }
}

#[derive(Debug)]
pub struct TypeInfoManager {
    memo: OnceMap<ir::Type, TypeInfo>,
}

impl TypeInfoManager {
    pub fn new() -> Self {
        Self {
            memo: OnceMap::new(),
        }
    }

    pub fn get_type_info(
        &self,
        ir_type: &ir::Type,
        target_info: &TargetInfo,
    ) -> Result<TypeInfo, ()> {
        self.memo.get_or_try_insert_ref(
            ir_type,
            (),
            |ty| ty.clone(),
            |_ctx, key| {
                let type_info = self.get_type_info_impl(key, target_info)?;
                Ok((type_info, type_info))
            },
            |_ctx, _key, value| *value,
        )
    }

    fn get_type_info_impl(
        &self,
        ir_type: &ir::Type,
        target_info: &TargetInfo,
    ) -> Result<TypeInfo, ()> {
        match ir_type {
            ir::Type::Pointer(_) | ir::Type::FunctionPointer => Ok(target_info.pointer_layout()),
            ir::Type::Boolean => Ok(target_info.bool_layout()),
            ir::Type::S8 | ir::Type::U8 => Ok(TypeInfo::basic(1)),
            ir::Type::S16 | ir::Type::U16 => Ok(TypeInfo::basic(2)),
            ir::Type::S32 | ir::Type::U32 => Ok(TypeInfo::basic(4)),
            ir::Type::S64 | ir::Type::U64 => Ok(TypeInfo::basic(8)),
            ir::Type::F32 => Ok(TypeInfo::basic(4)),
            ir::Type::F64 => Ok(TypeInfo::basic(8)),
            ir::Type::Void => Ok(TypeInfo {
                width_bytes: 0,
                align_bytes: 1,
            }),
            ir::Type::Structure(_structure_ref) => {
                /*
                let record_layout = RecordLayout::new(structure_ref);
                Ok(record_layout.type_info)
                */
                todo!("get_type_info_impl for ir::Type::Structure")
            }
            ir::Type::AnonymousComposite(_type_composite) => {
                /*
                let record_layout = RecordLayout::new(structure_ref);
                Ok(record_layout.type_info)
                */
                todo!("get_type_info_impl for ir::Type::AnonymousComposite")
            }
            ir::Type::FixedArray(fixed_array) => {
                let element_info = self.get_type_info(&fixed_array.inner, target_info)?;

                Ok(TypeInfo {
                    width_bytes: fixed_array.size * element_info.width_bytes,
                    align_bytes: element_info.align_bytes,
                })
            }
        }
    }
}
