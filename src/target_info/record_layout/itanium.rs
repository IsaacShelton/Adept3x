use super::{
    record_info::{FieldsIter, RecordInfo},
    FieldOffset,
};
use crate::{
    data_units::{BitUnits, ByteUnits},
    ir, resolved,
    target_info::{type_info::TypeInfoManager, TargetInfo},
};

#[derive(Clone, Debug)]
pub struct ItaniumRecordLayoutBuilder<'a> {
    pub type_info_manager: &'a TypeInfoManager<'a>,
    pub target_info: &'a TargetInfo,
    pub size: ByteUnits,
    pub alignment: ByteUnits,
    pub preferred_alignment: ByteUnits,
    pub unpacked_alignment: ByteUnits,
    pub unadjusted_alignment: ByteUnits,
    pub field_offsets: Vec<FieldOffset>,
    pub packed: bool,
    pub is_union: bool,
    pub is_natural_align: bool,
    pub is_ms_struct: bool,

    // Amount of left-over bits if the previous field was a bitfield (otherwise 0)
    pub unfilled_bits_in_last_unit: BitUnits,

    // When is_ms_struct, this is size of the storage unit of the previous field if it was a bitfield (otherwise 0)
    pub last_bitfield_storage_unit_size: BitUnits,

    pub max_field_alignment: ByteUnits,

    /// Size without tail padding
    pub data_size: ByteUnits,

    pub non_virtual_size: ByteUnits,
    pub non_virtual_alignment: ByteUnits,
    pub preferred_non_virtual_alignment: ByteUnits,
    pub padded_field_size: ByteUnits,

    pub primary_base: Option<&'a resolved::Structure>,
    pub has_packed_field: bool,
}

fn is_potentially_overlapping(field: &ir::Field) -> bool {
    field.properties.is_no_unique_addr && field.is_cxx_record()
}

impl<'a> ItaniumRecordLayoutBuilder<'a> {
    pub fn new(type_info_manager: &'a TypeInfoManager, target_info: &'a TargetInfo) -> Self {
        Self {
            type_info_manager,
            target_info,
            size: ByteUnits::of(0),
            alignment: ByteUnits::of(1),
            preferred_alignment: ByteUnits::of(1),
            unpacked_alignment: ByteUnits::of(1),
            unadjusted_alignment: ByteUnits::of(1),
            field_offsets: Default::default(),
            packed: false,
            is_union: false,
            is_natural_align: true,
            is_ms_struct: false,
            unfilled_bits_in_last_unit: BitUnits::of(0),
            last_bitfield_storage_unit_size: BitUnits::of(0),
            max_field_alignment: ByteUnits::of(0),
            data_size: ByteUnits::of(0),
            non_virtual_size: ByteUnits::of(0),
            non_virtual_alignment: ByteUnits::of(0),
            preferred_non_virtual_alignment: ByteUnits::of(0),
            padded_field_size: ByteUnits::of(0),
            primary_base: None,
            has_packed_field: false,
        }
    }

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

    pub fn layout_field(&mut self, field: &ir::Field, _insert_extra_padding: bool) {
        let field_class = field.as_cxx_record();
        let is_overlapping_empty_field = is_potentially_overlapping(field)
            && field_class.map_or(false, |class| class.is_empty());

        let _field_offset = if self.is_union || is_overlapping_empty_field {
            ByteUnits::of(0)
        } else {
            self.data_size
        };

        assert!(!field.is_bitfield(), "Bitfields not supported yet");

        let _unpadded_field_offset =
            BitUnits::from(self.data_size) - self.unfilled_bits_in_last_unit;

        todo!("layout field")
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
