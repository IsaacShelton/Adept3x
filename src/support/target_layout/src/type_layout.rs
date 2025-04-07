use data_units::ByteUnits;

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct TypeLayout {
    pub width: ByteUnits,
    pub alignment: ByteUnits,
    pub unadjusted_alignment: ByteUnits,
    pub align_requirement: AlignRequirement,
}

impl TypeLayout {
    pub fn basic(size: ByteUnits) -> Self {
        Self {
            width: size,
            alignment: size,
            unadjusted_alignment: size,
            align_requirement: AlignRequirement::None,
        }
    }
}

#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub enum AlignRequirement {
    #[default]
    None,
    RequiredByTypedefAttribute,
    RequiredByRecordAttribute,
}
