use crate::{Lexer, feed_result::FeedResult};
use token::Token;
use util_infinite_iterator::InfiniteIterator;
use util_text::Lexable;

impl<L, S> InfiniteIterator for Lexer<L, S>
where
    L: Lexable<S> + Send,
    S: Copy,
{
    type Item = Token<S>;

    fn next(&mut self) -> Self::Item {
        loop {
            if let FeedResult::Has(token) = self.feed() {
                return token;
            }
        }
    }
}
