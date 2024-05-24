mod character;
mod into_text;
mod into_text_stream;
mod text_stream;

use crate::ast::Source;

pub use character::{is_c_non_digit, Character};
pub use into_text::IntoText;
pub use into_text_stream::IntoTextStream;
pub use text_stream::TextStream;

/*
   General representation of incoming text.

   Generally, you don't implement this trait directly. Instead,
   you implement `TextStream`, and use the `IntoText` trait to
   create an easy to use text stream.

   This trait just provides nice wrappers around `TextStream`
*/
pub trait Text: TextStream {
    fn peek_nth(&mut self, n: usize) -> Character;

    fn peek(&mut self) -> Character {
        self.peek_nth(0)
    }

    fn eat(&mut self, expected: &str) -> bool {
        self.eat_remember(expected).is_ok()
    }

    fn eat_remember(&mut self, expected: &str) -> Result<Source, Source> {
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

    fn source(&mut self) -> Source {
        self.peek().source()
    }
}
