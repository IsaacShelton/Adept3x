use crate::line_column::Location;

pub trait IsCharacter {
    fn is_character(&self, c: char) -> bool;
}

impl IsCharacter for Option<(char, Location)> {
    fn is_character(&self, other_c: char) -> bool {
        self.map_or(false, |(c, _)| c == other_c)
    }
}

impl IsCharacter for Option<&(char, Location)> {
    fn is_character(&self, other_c: char) -> bool {
        self.map_or(false, |(c, _)| *c == other_c)
    }
}

impl IsCharacter for Option<&char> {
    fn is_character(&self, other_c: char) -> bool {
        self.map_or(false, |c| *c == other_c)
    }
}
