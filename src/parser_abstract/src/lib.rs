trait Parser {
    type State;
    type Output;

    fn parse(
        &self,
        starting_state: Option<Self::State>,
        tokens: impl InfiniteIterator<Token>,
    ) -> WithDiagnostics<Output>;
}

impl Parser for AdeptParser {
    type State = AdeptParserState;
    type Output = BareSyntaxTree;
}

impl Parser for CParser {}

impl Parser for AonParser {}
