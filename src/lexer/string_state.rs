use crate::{line_column::Location, token::StringModifier};

pub struct StringState {
    pub value: String,
    pub closing_char: char,
    pub modifier: StringModifier,
    pub start_location: Location,
}

