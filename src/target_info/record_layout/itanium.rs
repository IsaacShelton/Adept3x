use super::{
    record_info::{FieldsIter, RecordInfo},
    FieldOffset,
};
use crate::{
    ir, resolved,
    target_info::{type_info::TypeInfoManager, TargetInfo},
};

#[derive(Clone, Debug)]
pub struct ItaniumRecordLayoutBuilder<'a> {
    pub type_info_manager: &'a TypeInfoManager<'a>,
    pub target_info: &'a TargetInfo,
    pub size_bytes: u64,
    pub align_bytes: u64,
    pub preferred_align_bytes: u64,
    pub unpacked_align_bytes: u64,
    pub unadjusted_align_bytes: u64,
    pub field_offsets: Vec<FieldOffset>,
    pub packed: bool,
    pub is_union: bool,
    pub is_natural_align: bool,
    pub is_ms_struct: bool,
    pub max_field_align_bytes: u64,

    /// Size without tail padding
    pub data_size_bytes: u64,

    pub non_virtual_size_bytes: u64,
    pub non_virtual_align_bytes: u64,
    pub preferred_non_virtual_align_bytes: u64,
    pub padded_field_size_bytes: u64,

    pub primary_base: &'a resolved::Structure,
    pub has_packed_field: bool,
}

impl<'a> ItaniumRecordLayoutBuilder<'a> {
    pub fn layout<'t, F: FieldsIter<'t>>(&mut self, record: &'t RecordInfo<'t, F>) {
        // NOTE: This only works for C types
        self.init_layout();
        self.layout_fields(record);
        self.finish_layout(record);
    }

    pub fn init_layout(&mut self) {
        todo!()
    }

    pub fn layout_fields<'t, F: FieldsIter<'t>>(&mut self, record: &'t RecordInfo<'t, F>) {
        let insert_extra_padding = record.may_insert_extra_padding(true);
        let has_flexible_array_member = false; // NOTE: We don't support flexible array members yet

        for (i, field) in record.iter().enumerate() {
            let has_next = i + 1 < record.len();
            let insert_extra_padding_here =
                insert_extra_padding && (has_next || !has_flexible_array_member);

            self.layout_field(field, insert_extra_padding_here);
        }
    }

    pub fn layout_field(&mut self, _field: &ir::Field, _insert_extra_padding: bool) {
        todo!()
    }

    pub fn finish_layout<'t, F: FieldsIter<'t>>(&mut self, _record: &'t RecordInfo<'t, F>) {
        todo!()
    }
}

impl resolved::Structure {
    pub fn may_insert_extra_padding(&self, _emit_remark: bool) -> bool {
        todo!()
    }
}
