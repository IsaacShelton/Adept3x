use crate::ir;
use source_files::Source;

#[derive(Clone, Debug)]
pub struct RecordInfo<'env> {
    pub fields: &'env [ir::Field<'env>],
    pub is_packed: bool,
    pub is_union: bool,
    pub is_natural_align: bool,
    pub cxx_info: Option<()>,
    pub source: Source,
}

impl<'env> RecordInfo<'env> {
    pub fn from_struct(structure: &'env ir::Struct) -> Self {
        RecordInfo {
            fields: structure.fields,
            is_packed: structure.is_packed,
            is_union: false,
            is_natural_align: false,
            cxx_info: None,
            source: structure.source,
        }
    }

    pub fn from_composite(composite: &'env ir::TypeComposite) -> Self {
        RecordInfo {
            fields: composite.fields,
            is_packed: composite.is_packed,
            is_union: false,
            is_natural_align: false,
            cxx_info: None,
            source: composite.source,
        }
    }

    pub fn try_from_type(ir_type: &'env ir::Type, ir_module: &'env ir::Ir<'env>) -> Option<Self> {
        match ir_type {
            ir::Type::Union(_) => {
                todo!("RecordInfo::try_from_type for unions is not supported yet")
            }
            ir::Type::Struct(struct_ref) => {
                Some(RecordInfo::from_struct(&ir_module.structs[*struct_ref]))
            }
            ir::Type::AnonymousComposite(composite) => Some(RecordInfo::from_composite(composite)),
            _ => None,
        }
    }
}

impl<'env> RecordInfo<'env> {
    pub fn iter<'a>(&'a self) -> impl Iterator<Item = &'env ir::Field<'env>> + use<'a, 'env> {
        self.fields.iter()
    }

    pub fn len(&self) -> usize {
        self.fields.len()
    }

    pub fn may_insert_extra_padding(&self, _emit_remark: bool) -> bool {
        // NOTE: We don't support ASAN yet, so this will always be false
        false
    }
}
