use super::identifier_state::IdentifierState;

pub enum State {
    EndOfFile,
    Idle,
    Identifier(IdentifierState),
    // String(StringState),
    // Number(NumberState),
    // HexNumber(HexNumberState),
}
