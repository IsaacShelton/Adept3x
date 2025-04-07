use crate::Type;
use data_units::ByteUnits;
use derivative::Derivative;
use source_files::Source;

#[derive(Derivative, Clone, Debug)]
#[derivative(Hash, PartialEq, Eq)]
pub struct Field {
    pub ir_type: Type,
    pub properties: FieldProperties,

    #[derivative(PartialEq = "ignore")]
    #[derivative(Hash = "ignore")]
    pub source: Source,
}

impl Field {
    pub fn basic(ir_type: Type, source: Source) -> Self {
        Self {
            ir_type,
            properties: FieldProperties::default(),
            source,
        }
    }

    pub fn ir_type(&self) -> &Type {
        &self.ir_type
    }

    pub fn is_cxx_record(&self) -> bool {
        false
    }

    pub fn as_cxx_record_field(&self) -> Option<CXXRecordField> {
        None
    }

    pub fn is_bitfield(&self) -> bool {
        // NOTE: We don't support bitfields yet
        false
    }

    pub fn is_unnamed(&self) -> bool {
        self.properties.is_unnamed
    }

    pub fn is_zero_length_bitfield(&self) -> bool {
        // We don't support bitfields yet, but this will need to change
        // once we do
        self.is_bitfield() && todo!("is_zero_length_bitfield")
    }

    /// Returns the maximum alignment applied to the field (or 0 if unmodified)
    pub fn get_max_alignment(&self) -> ByteUnits {
        // NOTE: We don't support using `alignas` / `_Alignas` / GNU `aligned` / MSVC declspec `align`
        // on fields yet.
        // When we do, we will need to take the maximum value assigned, and return it here.
        ByteUnits::of(0)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CXXRecordField {}

impl CXXRecordField {
    pub fn is_empty(&self) -> bool {
        todo!("is_empty for c++ records not supported yet")
    }

    pub fn is_cxx_pod(&self) -> bool {
        todo!("is_cxx_pod for c++ records not supported yet")
    }

    pub fn is_packed(&self) -> bool {
        todo!("is_packed for c++ records not supported yet")
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FieldProperties {
    pub is_no_unique_addr: bool,
    pub is_force_packed: bool,
    pub is_unnamed: bool,
}

#[allow(clippy::derivable_impls)]
impl Default for FieldProperties {
    fn default() -> Self {
        Self {
            is_no_unique_addr: false,
            is_force_packed: false,
            is_unnamed: false,
        }
    }
}
