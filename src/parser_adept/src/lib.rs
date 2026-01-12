use parser_abstract::Parser;
use util_infinite_iterator::InfiniteIterator;

struct AdeptParser {}

enum AdeptParserState {
    Root,
}

impl Parser for AdeptParser {
    type State = AdeptParserState;
    type Token = ();
    type Output = ();

    fn parse(
        &self,
        starting_state: Option<Self::State>,
        tokens: impl InfiniteIterator<Item = Self::Token>,
    ) -> Self::Output {
        let state = starting_state.unwrap_or(AdeptParserState::Root);
        todo!("AdeptParser::parse")
    }
}
