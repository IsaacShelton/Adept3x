use crate::{source_files::Source, token::StringModifier};

pub struct StringState {
    pub value: String,
    pub closing_char: char,
    pub modifier: StringModifier,
    pub start_source: Source,
}
