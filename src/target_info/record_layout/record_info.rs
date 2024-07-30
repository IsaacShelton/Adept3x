use crate::{ast::Source, ir};

#[derive(Clone, Debug)]
pub struct RecordInfo<'t> {
    pub fields: &'t [ir::Field],
    pub is_packed: bool,
    pub is_union: bool,
    pub is_natural_align: bool,
    pub cxx_info: Option<()>,
    pub source: Source,
}

pub fn info_from_structure(structure: &ir::Structure) -> RecordInfo {
    RecordInfo {
        fields: &structure.fields[..],
        is_packed: structure.is_packed,
        is_union: false,
        is_natural_align: false,
        cxx_info: None,
        source: structure.source,
    }
}

pub fn info_from_composite(composite: &ir::TypeComposite) -> RecordInfo {
    RecordInfo {
        fields: &composite.fields[..],
        is_packed: composite.is_packed,
        is_union: false,
        is_natural_align: false,
        cxx_info: None,
        source: composite.source,
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
