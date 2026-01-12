use util_infinite_iterator::InfiniteIterator;

pub trait Parser {
    type State;
    type Token;
    type Output;

    fn parse(
        &self,
        starting_state: Option<Self::State>,
        tokens: impl InfiniteIterator<Item = Self::Token>,
    ) -> Self::Output;
}
