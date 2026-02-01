#[derive(Clone, Debug, PartialEq)]
pub struct StringLiteral {
    pub literal: String,
}

impl StringLiteral {
    pub fn modifier(&self) -> StringModifier {
        if self.literal.starts_with('"') {
            return StringModifier::Normal;
        }

        if self.literal.starts_with('\'') {
            return StringModifier::Character;
        }

        panic!("Invalid string literal")
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum StringModifier {
    Normal,
    Character,
}
