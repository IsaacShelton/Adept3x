mod itanium;

pub use itanium::Itanium;

pub enum CGCXXABI<'a> {
    Itanium(Itanium<'a>),
}

