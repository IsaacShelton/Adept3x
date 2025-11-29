use token::StringModifier;

pub struct StringState<S: Copy> {
    pub value: String,
    pub closing_char: char,
    pub modifier: StringModifier,
    pub start_source: S,
}
