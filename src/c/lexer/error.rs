use std::fmt::Display;

#[derive(Clone, Debug, PartialEq)]
pub enum LexError {
    UniversalCharacterNameNotSupported,
    UnrecognizedSymbol,
    UnrepresentableInteger,
}

impl Display for LexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LexError::UniversalCharacterNameNotSupported => {
                write!(f, "unsupported universal character name")
            }
            LexError::UnrecognizedSymbol => write!(f, "unrecognized symbol"),
            LexError::UnrepresentableInteger => write!(f, "unrepresentable integer literal"),
        }
    }
}
