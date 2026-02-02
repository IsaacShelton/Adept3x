mod character;
mod column_spacing_atom;
mod eatable;
mod line_spacing_atom;
mod peeker;

pub use character::Character;
pub use column_spacing_atom::ColumnSpacingAtom;
pub use eatable::Eatable;
pub use line_spacing_atom::LineSpacingAtom;
pub use peeker::Peeker as CharacterPeeker;
use util_infinite_iterator::InfiniteIterator;

pub trait Lexable<S: Copy>: InfiniteIterator<Item = Character<S>> {
    fn peek_nth(&mut self, n: usize) -> Self::Item;

    fn peek_n<const N: usize>(&mut self) -> [Self::Item; N] {
        std::array::from_fn(|i| self.peek_nth(i))
    }

    fn peek(&mut self) -> Self::Item {
        self.peek_nth(0)
    }

    fn peek_starts_with(&mut self, pattern: impl Eatable) -> bool {
        for (i, c) in pattern.chars().enumerate() {
            if !self.peek_nth(i).is(c) {
                return false;
            }
        }
        true
    }

    fn eat_line_spacing_atom(&mut self) -> Option<(LineSpacingAtom, S)> {
        if let Character::At(c @ '\n', source) = self.peek() {
            let mut count = 0;

            loop {
                if !self.eat(c) {
                    break;
                }

                count += 1;
            }

            return Some((LineSpacingAtom { count }, source));
        }

        None
    }

    fn eat_column_spacing_atom(&mut self) -> Option<(ColumnSpacingAtom, S)> {
        if let Character::At(c @ (' ' | '\t'), source) = self.peek() {
            let mut count = 0;

            loop {
                if !self.eat(c) {
                    break;
                }

                count += 1;
            }

            let atom = match c {
                ' ' => ColumnSpacingAtom::Spaces(count.try_into().unwrap()),
                '\t' => ColumnSpacingAtom::Tabs(count.try_into().unwrap()),
                _ => unreachable!(),
            };

            return Some((atom, source));
        }

        None
    }

    fn eat(&mut self, expected: impl Eatable) -> bool {
        self.eat_remember(expected).is_ok()
    }

    fn eat_remember(&mut self, expected: impl Eatable) -> Result<S, S> {
        let start = self.peek().source();
        let mut count = 0;

        // Check if matches
        for (i, expected_c) in expected.chars().enumerate() {
            match self.peek_nth(i) {
                Character::At(c, _) => {
                    if c != expected_c {
                        return Err(start);
                    }
                    count += 1;
                }
                Character::End(_) => return Err(start),
            }
        }

        // Consume the match
        for _ in 0..count {
            self.next();
        }

        Ok(start)
    }
}
