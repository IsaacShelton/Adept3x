use util_infinite_iterator::IsEnd;

#[derive(Clone, Debug)]
pub enum Character<S: Copy> {
    At(char, S),
    End(S),
}

impl<S: Copy> Character<S> {
    #[inline]
    pub fn or_nul(self) -> char {
        match self {
            Character::At(c, _) => c,
            Character::End(_) => '\0',
        }
    }

    #[inline]
    pub fn unwrap(&self) -> (char, S) {
        match self {
            Character::At(c, source) => (*c, *source),
            Character::End(_) => panic!("unwrap of end character"),
        }
    }

    #[inline]
    pub fn expect(&self, message: &str) -> (char, S) {
        match self {
            Character::At(c, source) => (*c, *source),
            Character::End(_) => panic!("{}", message),
        }
    }

    #[inline]
    pub fn is(&self, character: char) -> bool {
        match self {
            Character::At(c, _) => *c == character,
            Character::End(_) => false,
        }
    }

    #[inline]
    pub fn is_digit(&self) -> bool {
        match self {
            Character::At(c, _) => c.is_ascii_digit(),
            Character::End(_) => false,
        }
    }

    #[inline]
    pub fn is_identifier_start(&self) -> bool {
        match self {
            Character::At(c, _) => c.is_ascii_alphabetic() || *c == '_',
            Character::End(_) => false,
        }
    }

    #[inline]
    pub fn is_identifier_continue(&self) -> bool {
        match self {
            Character::At(c, _) => c.is_ascii_alphabetic() || c.is_ascii_digit() || *c == '_',
            Character::End(_) => false,
        }
    }

    #[inline]
    pub fn is_c_non_digit(&self) -> bool {
        // NOTE: We support the extension of using '$' in identifier/non-digit character
        match self {
            Character::At(c, _) => is_c_non_digit(*c),
            Character::End(_) => false,
        }
    }

    #[inline]
    pub fn is_sign(&self) -> bool {
        matches!(self, Character::At('+' | '-', _))
    }

    #[inline]
    pub fn is_space_or_tab(&self) -> bool {
        matches!(self, Character::At(' ' | '\t', _))
    }

    #[inline]
    pub fn is_space_or_tab_or_newline(&self) -> bool {
        matches!(self, Character::At(' ' | '\t' | '\n', _))
    }

    #[inline]
    pub fn source(&self) -> S {
        match self {
            Character::At(_, source) => *source,
            Character::End(source) => *source,
        }
    }

    #[inline]
    pub fn is_end(&self) -> bool {
        match self {
            Character::At(..) => false,
            Character::End(_) => true,
        }
    }

    #[inline]
    pub fn is_present(&self) -> bool {
        !self.is_end()
    }
}

pub fn is_c_non_digit(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_' || c == '$'
}

impl<S: Copy> IsEnd for Character<S> {
    fn is_end(&self) -> bool {
        matches!(self, Self::End(..))
    }
}
