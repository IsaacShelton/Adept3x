use super::{
    compound_identifier_state::CompoundIdentifierState, hex_number_state::HexNumberState,
    identifier_state::IdentifierState, number_state::NumberState, polymorph_state::PolymorphState,
    string_state::StringState,
};
use crate::macro_state::MacroState;
use derive_more::Unwrap;

#[derive(Unwrap)]
pub enum State<S: Copy> {
    Idle,
    Identifier(IdentifierState<S>),
    CompoundIdentifier(CompoundIdentifierState<S>),
    Polymorph(PolymorphState<S>),
    Macro(MacroState<S>),
    String(StringState<S>),
    Number(NumberState<S>),
    HexNumber(HexNumberState<S>),
    ShortGeneric,
}

impl<S: Copy> State<S> {
    pub fn as_mut_identifier(&mut self) -> &mut IdentifierState<S> {
        match self {
            State::Identifier(identifier) => identifier,
            _ => panic!(),
        }
    }

    pub fn as_mut_compound_identifier(&mut self) -> &mut CompoundIdentifierState<S> {
        match self {
            State::CompoundIdentifier(compound_identifier) => compound_identifier,
            _ => panic!(),
        }
    }

    pub fn as_mut_polymorph(&mut self) -> &mut PolymorphState<S> {
        match self {
            State::Polymorph(polymorph) => polymorph,
            _ => panic!(),
        }
    }

    pub fn as_mut_macro(&mut self) -> &mut MacroState<S> {
        match self {
            State::Macro(macro_state) => macro_state,
            _ => panic!(),
        }
    }

    pub fn as_mut_string(&mut self) -> &mut StringState<S> {
        match self {
            State::String(state) => state,
            _ => panic!(),
        }
    }

    pub fn as_mut_number(&mut self) -> &mut NumberState<S> {
        match self {
            State::Number(state) => state,
            _ => panic!(),
        }
    }

    pub fn as_mut_hex_number(&mut self) -> &mut HexNumberState<S> {
        match self {
            State::HexNumber(state) => state,
            _ => panic!(),
        }
    }
}
