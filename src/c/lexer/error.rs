#[derive(Clone, Debug, PartialEq)]
pub enum LexError {
    UniversalCharacterNameNotSupported,
    UnrecognizedSymbol,
    UnrepresentableInteger,
}
