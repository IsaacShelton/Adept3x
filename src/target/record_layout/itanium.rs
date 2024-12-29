use super::record_info::RecordInfo;
use crate::{
    asg,
    data_units::{BitUnits, ByteUnits},
    diagnostics::{Diagnostics, WarningDiagnostic},
    ir,
    target::{
        type_layout::{ASTRecordLayout, TypeLayoutCache},
        TargetOsExt,
    },
};

#[derive(Debug)]
pub struct ItaniumRecordLayoutBuilder<'a> {
    pub type_layout_cache: &'a TypeLayoutCache<'a>,
    pub empty_subobjects: Option<&'a mut EmptySubobjects>,
    pub size: BitUnits,
    pub alignment: ByteUnits,
    pub preferred_alignment: ByteUnits,
    pub unpacked_alignment: ByteUnits,
    pub unadjusted_alignment: ByteUnits,
    pub field_offsets: Vec<BitUnits>,
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
    pub data_size: BitUnits,

    pub non_virtual_size: ByteUnits,
    pub non_virtual_alignment: ByteUnits,
    pub preferred_non_virtual_alignment: ByteUnits,
    pub padded_field_size: BitUnits,

    pub primary_base: Option<&'a asg::Structure>,
    pub has_packed_field: bool,

    // NOTE: We don't support using external layouts / inferring alignments yet
    pub use_external_layout: bool,
    pub infer_alignment: bool,

    pub friendly_record_name: &'a str,
    pub diagnostics: &'a Diagnostics<'a>,
}

impl<'a> ItaniumRecordLayoutBuilder<'a> {
    pub fn generate(
        type_layout_cache: &'a TypeLayoutCache,
        diagnostics: &'a Diagnostics,
        info: &RecordInfo,
        name: Option<&'a str>,
    ) -> ASTRecordLayout {
        let mut builder =
            ItaniumRecordLayoutBuilder::new(type_layout_cache, None, diagnostics, name);
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

    pub fn new(
        type_layout_cache: &'a TypeLayoutCache,
        empty_subobjects: Option<&'a mut EmptySubobjects>,
        diagnostics: &'a Diagnostics,
        friendly_record_name: Option<&'a str>,
    ) -> Self {
        Self {
            type_layout_cache,
            empty_subobjects,
            size: BitUnits::of(0),
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
            data_size: BitUnits::of(0),
            non_virtual_size: ByteUnits::of(0),
            non_virtual_alignment: ByteUnits::of(0),
            preferred_non_virtual_alignment: ByteUnits::of(0),
            padded_field_size: BitUnits::of(0),
            primary_base: None,
            has_packed_field: false,
            use_external_layout: false, // We don't support using external layouts yet
            infer_alignment: false,
            friendly_record_name: friendly_record_name.unwrap_or("<unnamed composite>"),
            diagnostics,
        }
    }

    pub fn layout(&mut self, record: &RecordInfo) {
        // NOTE: This only works for C types
        self.init_layout(record);
        self.layout_fields(record);
        self.finish_layout(record);
    }

    pub fn init_layout(&mut self, record: &RecordInfo) {
        self.is_union = record.is_union;
        self.is_ms_struct = false;
        self.packed = record.is_packed;

        if record.is_natural_align {
            self.is_natural_align = true;
        }

        // NOTE: We don't allow alignment attributes on records yet,
        // it would require extra alignment work here.

        // We don't really care about anything else for now...
    }

    pub fn layout_fields(&mut self, record: &RecordInfo) {
        let insert_extra_padding = record.may_insert_extra_padding(true);
        let has_flexible_array_member = false; // NOTE: We don't support flexible array members yet

        for (i, field) in record.iter().enumerate() {
            let has_next = i + 1 < record.len();
            let insert_extra_padding_here =
                insert_extra_padding && (has_next || !has_flexible_array_member);

            self.layout_field(field, insert_extra_padding_here, i);
        }
    }

    pub fn layout_field(&mut self, field: &ir::Field, insert_extra_padding: bool, field_i: usize) {
        let field_class = field.as_cxx_record();
        let is_overlapping_empty_field = is_potentially_overlapping(field)
            && field_class.as_ref().map_or(false, |class| class.is_empty());

        let field_offset = if self.is_union || is_overlapping_empty_field {
            BitUnits::of(0)
        } else {
            self.data_size
        };

        assert!(!field.is_bitfield(), "Bitfields not supported yet");

        let unpadded_field_offset = self.data_size - self.unfilled_bits_in_last_unit;

        self.unfilled_bits_in_last_unit = BitUnits::of(0);
        self.last_bitfield_storage_unit_size = BitUnits::of(0);

        let type_layout = self.type_layout_cache.get(&field.ir_type);
        let mut field_alignment = type_layout.alignment;

        let mut field_size = if field.ir_type.is_incomplete_array() {
            ByteUnits::of(0)
        } else {
            type_layout.width
        };

        let mut effective_field_size = field_size;

        if field.ir_type.is_incomplete_array() {
            field_size = ByteUnits::of(0);
            effective_field_size = ByteUnits::of(0);
        } else {
            if is_potentially_overlapping(field) {
                todo!("ItaniumRecordLayoutBuilder::layout_field for is_potentially_overlapping is not fully implemented yet");

                /*
                let record_layout = ASTRecordLayout::new(
                    field_class.expect("C++ class if is_potentially_overlapping"),
                );

                effective_field_size = record_layout
                    .non_virtual_size()
                    .max(record_layout.data_size());
                */
            }

            if self.is_ms_struct {
                todo!("is_ms_struct bitfield shenanigans");
            }
        }

        let field_packed = (self.packed
            && (field_class.as_ref().map_or(true, |field_class| {
                field_class.is_cxx_pod()
                    || field_class.is_packed()
                    || self.type_layout_cache.target.os.is_mac()
            })))
            || field.properties.is_force_packed;

        let preferred_alignment = field_alignment;

        // Calculate regular alignment for when not packed, can be used for warning message
        let unpacked_field_alignment = field_alignment;
        let packed_field_alignment = ByteUnits::of(1);

        let unpacked_field_offset = field_offset;
        let max_alignment = field.get_max_alignment();

        let mut packed_field_alignment = packed_field_alignment.max(max_alignment);
        let mut preferred_alignment = preferred_alignment.max(max_alignment);
        let mut unpacked_field_alignment = unpacked_field_alignment.max(max_alignment);

        if !max_alignment.is_zero() {
            packed_field_alignment = packed_field_alignment.min(self.max_field_alignment);
            preferred_alignment = preferred_alignment.min(self.max_field_alignment);
            unpacked_field_alignment = unpacked_field_alignment.min(self.max_field_alignment);
        }

        if !field_packed {
            field_alignment = unpacked_field_alignment;
        } else {
            preferred_alignment = packed_field_alignment;
            field_alignment = packed_field_alignment;
        }

        let align_to = field_alignment;
        let mut field_offset = field_offset.align_to(BitUnits::from(align_to));

        let unpacked_field_offset =
            unpacked_field_offset.align_to(BitUnits::from(unpacked_field_alignment));

        if !self.use_external_layout && !self.is_union {
            if let Some(empty_subobjects) = self.empty_subobjects.as_mut() {
                // Check if we can place this field at this offset
                while !empty_subobjects.can_place_field_at_offset(field, field_offset) {
                    // Failed to place at this offset (we first try to place at offset 0, then data size onwards)
                    if field_offset.is_zero() && !self.data_size.is_zero() {
                        field_offset = self.data_size.align_to(BitUnits::from(align_to));
                    } else {
                        field_offset += BitUnits::from(align_to);
                    }
                }
            }
        }

        assert!((field_offset % BitUnits::from(align_to)).is_zero());

        // Place field at current location
        self.field_offsets.push(field_offset);

        if !self.use_external_layout {
            self.check_field_padding(
                field_offset,
                unpadded_field_offset,
                unpacked_field_offset,
                field_packed,
                field,
                field_i,
            );
        }

        if insert_extra_padding {
            let asan_alignment = ByteUnits::of(8);
            let mut extra_size_for_asan = asan_alignment;
            if field_size % asan_alignment != ByteUnits::of(0) {
                extra_size_for_asan += asan_alignment - (field_size % asan_alignment);
            }

            field_size += extra_size_for_asan;
            effective_field_size = field_size;
        }

        // Reserve space for this field
        if !is_overlapping_empty_field {
            let effective_field_size = BitUnits::from(effective_field_size);

            if self.is_union {
                self.data_size = self.data_size.max(effective_field_size);
            } else {
                self.data_size = field_offset + effective_field_size;
            }

            self.padded_field_size = self
                .padded_field_size
                .max(field_offset + BitUnits::from(field_size));

            self.size = self.size.max(self.data_size);
        } else {
            self.size = self.size.max(field_offset + field_offset);
        }

        self.unadjusted_alignment = self.unadjusted_alignment.max(field_alignment);

        self.update_alignment(
            field_alignment,
            unpacked_field_alignment,
            preferred_alignment,
        );

        // NOTE: We don't support parent records yet
        let has_parent = false;
        if has_parent {
            todo!("ensure child record layout is compatible with parent layout");
        }

        if self.packed && !field_packed && packed_field_alignment < field_alignment {
            self.diagnostics.push(WarningDiagnostic::new(
                format!("Unpacked field in type '{}'", self.friendly_record_name),
                field.source,
            ));
        }
    }

    pub fn finish_layout(&mut self, record: &RecordInfo) {
        // NOTE: Records in C++ cannot be zero-sized
        if record.cxx_info.is_some() && self.size.is_zero() {
            todo!("zero-sized c++ records are not supported yet")
        }

        // Include final field's tail padding in total size
        self.size = self.size.max(self.padded_field_size);

        // Round size of record up to its alignment
        let unpadded_size = self.size - self.unfilled_bits_in_last_unit;
        let unpacked_size = self.size.align_to(BitUnits::from(self.unpacked_alignment));

        let rounded_size = self.size.align_to(BitUnits::from(self.preferred_alignment));

        assert!(
            !self.use_external_layout,
            "external layout not supported yet"
        );

        self.size = rounded_size;

        if self.size > unpadded_size {
            let pad_size = self.size - unpadded_size;

            self.diagnostics.push(WarningDiagnostic::new(
                format!(
                    "Padded type '{}', with {} bits to alignment boundary",
                    self.friendly_record_name,
                    pad_size.bits()
                ),
                record.source,
            ));
        }

        if self.packed
            && self.unpacked_alignment < self.alignment
            && unpacked_size == self.size
            && !self.has_packed_field
        {
            self.diagnostics.push(WarningDiagnostic::new(
                format!("Unnecessarily packed type '{}'", self.friendly_record_name),
                record.source,
            ));
        }
    }

    pub fn update_alignment(
        &mut self,
        new_alignment: ByteUnits,
        new_unpacked_alignment: ByteUnits,
        new_preferred_alignment: ByteUnits,
    ) {
        if self.use_external_layout || !self.infer_alignment {
            return;
        }

        if new_alignment > self.alignment {
            assert!(new_alignment.is_power_of_2());
            self.alignment = new_alignment;
        }

        if new_unpacked_alignment > self.unpacked_alignment {
            assert!(new_unpacked_alignment.is_power_of_2());
            self.alignment = new_unpacked_alignment;
        }

        if new_preferred_alignment > self.preferred_alignment {
            assert!(new_preferred_alignment.is_power_of_2());
            self.alignment = new_preferred_alignment;
        }
    }

    fn check_field_padding(
        &mut self,
        field_offset: BitUnits,
        unpadded_field_offset: BitUnits,
        unpacked_field_offset: BitUnits,
        field_packed: bool,
        field: &ir::Field,
        field_i: usize,
    ) {
        if !self.is_union && field_offset > unpadded_field_offset {
            if field.is_bitfield() {
                if self.diagnostics.flags().warn_padded_bitfield {
                    self.diagnostics.push(WarningDiagnostic::new(
                        format!(
                            "Padded bitfield of '{}' at index {}",
                            self.friendly_record_name, field_i,
                        ),
                        field.source,
                    ));
                }
            } else {
                if self.diagnostics.flags().warn_padded_field {
                    self.diagnostics.push(WarningDiagnostic::new(
                        format!(
                            "Padded field of '{}' at index {}",
                            self.friendly_record_name, field_i,
                        ),
                        field.source,
                    ));
                }
            }
        }

        if field_packed && field_offset != unpacked_field_offset {
            self.has_packed_field = true;
        }
    }
}

fn is_potentially_overlapping(field: &ir::Field) -> bool {
    field.properties.is_no_unique_addr && field.is_cxx_record()
}

/// Keeps track of the offsets of different empty subobjects for C++ (but not C) record layouts
#[derive(Debug)]
pub struct EmptySubobjects {}

impl EmptySubobjects {
    pub fn can_place_field_at_offset(&mut self, _field: &ir::Field, _offset: BitUnits) -> bool {
        todo!("can_place_field_at_offset for C++ records not implemented yet")
    }
}
