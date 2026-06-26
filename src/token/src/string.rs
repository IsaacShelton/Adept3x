use std::{fmt::Display, sync::Arc};

#[derive(Clone, Debug, PartialEq)]
pub struct StringLiteral {
    pub full_text: String,
}

impl StringLiteral {
    pub fn modifier(&self) -> StringModifier {
        if self.full_text.starts_with('"') {
            return StringModifier::Normal;
        }

        if self.full_text.starts_with('\'') {
            return StringModifier::Character;
        }

        panic!("Invalid string literal")
    }

    pub fn to_value(&self) -> Result<Arc<str>, ()> {
        // TODO: Replace this with custom string escaping logic
        snailquote::unescape(&self.full_text)
            .map(From::from)
            .map_err(|_| ())
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum StringModifier {
    Normal,
    Character,
}

impl Display for StringLiteral {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.full_text)
    }
}
