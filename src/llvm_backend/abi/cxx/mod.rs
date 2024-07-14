mod itanium;

pub use itanium::Itanium;

#[allow(clippy::upper_case_acronyms)]
pub enum CGCXXABI<'a> {
    Itanium(Itanium<'a>),
}
