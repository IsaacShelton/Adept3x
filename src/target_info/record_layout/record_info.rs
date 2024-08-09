use crate::{ir, source_files::Source};

#[derive(Clone, Debug)]
pub struct RecordInfo<'t> {
    pub fields: &'t [ir::Field],
    pub is_packed: bool,
    pub is_union: bool,
    pub is_natural_align: bool,
    pub cxx_info: Option<()>,
    pub source: Source,
}

impl<'t> RecordInfo<'t> {
    pub fn from_structure(structure: &'t ir::Structure) -> Self {
        RecordInfo {
            fields: &structure.fields[..],
            is_packed: structure.is_packed,
            is_union: false,
            is_natural_align: false,
            cxx_info: None,
            source: structure.source,
        }
    }

    pub fn from_composite(composite: &'t ir::TypeComposite) -> Self {
        RecordInfo {
            fields: &composite.fields[..],
            is_packed: composite.is_packed,
            is_union: false,
            is_natural_align: false,
            cxx_info: None,
            source: composite.source,
        }
    }

    pub fn try_from_type(value: &'t ir::Type, ir_module: &'t ir::Module) -> Option<Self> {
        match value {
            ir::Type::Union(_) => {
                todo!("RecordInfo::try_from_type for unions is not supported yet")
            }
            ir::Type::Structure(structure_ref) => ir_module
                .structures
                .get(structure_ref)
                .map(RecordInfo::from_structure),
            ir::Type::AnonymousComposite(composite) => Some(RecordInfo::from_composite(composite)),
            _ => None,
        }
    }
}

impl<'t> RecordInfo<'t> {
    pub fn iter(&self) -> impl Iterator<Item = &ir::Field> {
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
