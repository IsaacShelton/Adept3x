use super::{
    compound_identifier_state::CompoundIdentifierState, hex_number_state::HexNumberState,
    identifier_state::IdentifierState, number_state::NumberState, polymorph_state::PolymorphState,
    string_state::StringState,
};
use derive_more::Unwrap;

#[derive(Unwrap)]
pub enum State {
    Idle,
    Identifier(IdentifierState),
    CompoundIdentifier(CompoundIdentifierState),
    Polymorph(PolymorphState),
    String(StringState),
    Number(NumberState),
    HexNumber(HexNumberState),
    ShortGeneric,
}

impl State {
    pub fn as_mut_identifier(&mut self) -> &mut IdentifierState {
        match self {
            State::Identifier(identifier) => identifier,
            _ => panic!(),
        }
    }

    pub fn as_mut_compound_identifier(&mut self) -> &mut CompoundIdentifierState {
        match self {
            State::CompoundIdentifier(compound_identifier) => compound_identifier,
            _ => panic!(),
        }
    }

    pub fn as_mut_polymorph(&mut self) -> &mut PolymorphState {
        match self {
            State::Polymorph(polymorph) => polymorph,
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
