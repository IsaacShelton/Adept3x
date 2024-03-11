use super::{
    hex_number_state::HexNumberState, identifier_state::IdentifierState, number_state::NumberState,
    string_state::StringState,
};

pub enum State {
    EndOfFile,
    Idle,
    Identifier(IdentifierState),
    String(StringState),
    Number(NumberState),
    HexNumber(HexNumberState),
}

impl State {
    pub fn as_mut_identifier(&mut self) -> &mut IdentifierState {
        match self {
            State::Identifier(identifier) => identifier,
            _ => panic!(),
        }
    }

    pub fn as_mut_string(&mut self) -> &mut StringState {
        match self {
            State::String(state) => state,
            _ => panic!(),
        }
    }

    pub fn as_mut_number(&mut self) -> &mut NumberState {
        match self {
            State::Number(state) => state,
            _ => panic!(),
        }
    }

    pub fn as_mut_hex_number(&mut self) -> &mut HexNumberState {
        match self {
            State::HexNumber(state) => state,
            _ => panic!(),
        }
    }
}
