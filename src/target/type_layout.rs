use super::{record_layout::record_info::RecordInfo, Target};
use crate::{
    asg::Asg,
    data_units::{BitUnits, ByteUnits},
    diagnostics::Diagnostics,
    ir,
    target::record_layout::itanium::ItaniumRecordLayoutBuilder,
};
use once_map::unsync::OnceMap;

#[derive(Copy, Clone, Debug, Default)]
pub enum AlignmentRequirement {
    #[default]
    None,
    RequiredByTypedefAttribute,
    RequiredByRecordAttribute,
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

#[derive(Copy, Clone, Debug)]
pub struct TypeLayout {
    pub width: ByteUnits,
    pub alignment: ByteUnits,
    pub unadjusted_alignment: ByteUnits,
    pub alignment_requirement: AlignmentRequirement,
}

impl TypeLayout {
    pub fn basic(size: ByteUnits) -> Self {
        Self {
            width: size,
            alignment: size,
            unadjusted_alignment: size,
            alignment_requirement: AlignmentRequirement::None,
        }
    }
}

#[derive(Debug)]
pub struct TypeLayoutCache<'a> {
    memo: OnceMap<ir::Type, TypeLayout>,
    pub target: &'a Target,
    pub structs: &'a ir::Structs,
    pub asg: &'a Asg<'a>,
    pub diagnostics: &'a Diagnostics<'a>,
}

impl<'a> TypeLayoutCache<'a> {
    pub fn new(
        target: &'a Target,
        structures: &'a ir::Structs,
        asg: &'a Asg,
        diagnostics: &'a Diagnostics<'a>,
    ) -> Self {
        Self {
            memo: OnceMap::new(),
            target,
            structs: structures,
            asg,
            diagnostics,
        }
    }

    pub fn get(&self, ir_type: &ir::Type) -> TypeLayout {
        self.memo.map_insert_ref(
            ir_type,
            |ty| ty.clone(),
            |key| self.get_impl(key),
            |_k, v| *v,
        )
    }

    fn get_impl(&self, ir_type: &ir::Type) -> TypeLayout {
        match ir_type {
            ir::Type::Pointer(_) | ir::Type::FunctionPointer => self.target.pointer_layout(),
            ir::Type::Boolean => self.target.bool_layout(),
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
                alignment_requirement: AlignmentRequirement::None,
            },
            ir::Type::Union(_) => todo!("get_impl for ir::Type::Union"),
            ir::Type::Structure(struct_ref) => {
                let structure = self.structs.get(*struct_ref);

                let info = RecordInfo::from_structure(structure);
                self.get_impl_record_layout(&info, structure.name.as_deref())
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
                    alignment_requirement: element_info.alignment_requirement,
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

    fn get_impl_record_layout(&self, info: &RecordInfo, name: Option<&str>) -> TypeLayout {
        // TODO: We should cache this

        let record_layout =
            ItaniumRecordLayoutBuilder::generate(self, self.diagnostics, info, name);

        // NOTE: We don't support alignment attributes yet,
        // so this will always be none
        let alignment_requirement = AlignmentRequirement::None;

        TypeLayout {
            width: record_layout.size,
            alignment: record_layout.alignment,
            alignment_requirement,
            unadjusted_alignment: record_layout.unadjusted_alignment,
        }
    }
}
