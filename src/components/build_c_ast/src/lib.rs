pub mod parse;
pub mod translate;

pub use self::translate::translate_expr;
use attributes::Privacy;

#[derive(Copy, Clone, Debug)]
pub enum CFileType {
    Header,
    Source,
}

impl CFileType {
    pub fn privacy(&self) -> Privacy {
        match self {
            CFileType::Header => Privacy::Protected,
            CFileType::Source => Privacy::Private,
        }
    }
}
