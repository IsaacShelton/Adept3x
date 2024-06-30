use crate::ir;

#[derive(Clone, Debug)]
pub struct RecordInfo<'t> {
    pub fields: &'t [ir::Field],
    pub is_packed: bool,
    pub is_union: bool,
    pub is_natural_align: bool,
    pub cxx_info: Option<()>,
}

pub fn info_from_structure<'t>(structure: &'t ir::Structure) -> RecordInfo<'t> {
    RecordInfo {
        fields: &structure.fields[..],
        is_packed: structure.is_packed,
        is_union: false,
        is_natural_align: false,
        cxx_info: None,
    }
}

pub fn info_from_composite<'t>(composite: &'t ir::TypeComposite) -> RecordInfo<'t> {
    RecordInfo {
        fields: &composite.fields[..],
        is_packed: composite.is_packed,
        is_union: false,
        is_natural_align: false,
        cxx_info: None,
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
