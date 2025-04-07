use data_units::ByteUnits;

#[derive(Clone, Debug)]
pub enum AvxLevel {
    None,
    Avx,
    Avx512,
}

impl AvxLevel {
    pub fn native_vector_size(&self) -> ByteUnits {
        ByteUnits::of(match self {
            AvxLevel::None => 16,
            AvxLevel::Avx => 32,
            AvxLevel::Avx512 => 64,
        })
    }
}
