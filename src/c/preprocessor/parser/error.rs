use crate::{ast::Source, c::preprocessor::pre_token::Punctuator, show::Show, source_file_cache::SourceFileCache};
use std::fmt::Display;

#[derive(Clone, Debug)]
pub struct ParseError {
    pub kind: ParseErrorKind,
    pub source: Source,
}

impl ParseError {
    pub fn new(kind: ParseErrorKind, source: Source) -> Self {
        Self { kind, source }
    }
}

impl Show for ParseError {
    fn show(
        &self,
        w: &mut impl std::fmt::Write,
        source_file_cache: &SourceFileCache,
    ) -> std::fmt::Result {
        write!(
            w,
            "{}:{}:{}: error: {}",
            source_file_cache.get(self.source.key).filename(),
            self.source.location.line,
            self.source.location.column,
            self.kind
        )
    }
}

#[derive(Clone, Debug)]
pub enum ParseErrorKind {
    // Initial parsing errors...
    ExpectedGroupPart,
    ExpectedMacroNameFor,
    UnexpectedToken { after: String },
    ExpectedEndif,
    UnrecognizedDirective(String),
    ExpectedDefinitionName,
    ExpectedNewlineAfterDirective,
    UnrecognizedPragmaDirective(String),
    UnrecognizedAdeptPragmaDirective,
    ExpectedOpenParen,
    ExpectedParameterName,
    ExpectedComma,
    ExpectedCloseParenAfterVarArgs,
    ExpectedPunctuator(Punctuator),
    // Expression parsing errors... (These occur during expansion)
    ExpectedExpression,
    BadInteger,
    ExpectedCloseParen,
    ExpectedColon,
    NotEnoughArguments,
    TooManyArguments,
    ExpectedOpenParenDuringExpansion,
    ExpectedEndOfExpression,
}

impl ParseErrorKind {
    pub fn at(self, source: Source) -> ParseError {
        ParseError::new(self, source)
    }
}

impl Display for ParseErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseErrorKind::ExpectedGroupPart => {
                f.write_str("Expected group part during preprocessing")
            }
            ParseErrorKind::ExpectedMacroNameFor => {
                f.write_str("Expected macro name for directive")
            }
            ParseErrorKind::UnexpectedToken { after } => {
                write!(f, "Unexpected token after {} during preprocessing", after)
            }
            ParseErrorKind::ExpectedEndif => f.write_str("unexpected #endif"),
            ParseErrorKind::UnrecognizedDirective(directive) => {
                write!(f, "Unrecognized preprocessing directive #{}", directive)
            }
            ParseErrorKind::ExpectedDefinitionName => {
                f.write_str("Expected definition name during preprocessing")
            }
            ParseErrorKind::ExpectedNewlineAfterDirective => {
                f.write_str("Expected newline after preprocessing directive")
            }
            ParseErrorKind::UnrecognizedPragmaDirective(directive) => {
                write!(f, "Unrecognized pragma directive {}", directive)
            }
            ParseErrorKind::UnrecognizedAdeptPragmaDirective => {
                write!(f, "Unrecognized adept pragma directive")
            }
            ParseErrorKind::ExpectedOpenParen => f.write_str("Expected '(' during preprocessing"),
            ParseErrorKind::ExpectedParameterName => {
                f.write_str("Expected parameter name during preprocessing")
            }
            ParseErrorKind::ExpectedComma => f.write_str("Expected ',' during preprocessing"),
            ParseErrorKind::ExpectedCloseParenAfterVarArgs => {
                f.write_str("Expected ')' after '...' in macro parameter list")
            }
            ParseErrorKind::ExpectedPunctuator(punctuator) => {
                write!(f, "Expected '{}' during preprocessing", punctuator)
            }
            ParseErrorKind::ExpectedExpression => {
                f.write_str("Expected expression during preprocessing")
            }
            ParseErrorKind::BadInteger => f.write_str("Bad integer during preprocessing"),
            ParseErrorKind::ExpectedCloseParen => f.write_str("Expected ')' during preprocessing"),
            ParseErrorKind::ExpectedColon => f.write_str("Expected ':' during preprocessing"),
            ParseErrorKind::NotEnoughArguments => {
                f.write_str("Not enough arguments to preprocessing macro")
            }
            ParseErrorKind::TooManyArguments => {
                f.write_str("Too many arguments to preprocessing macro")
            }
            ParseErrorKind::ExpectedOpenParenDuringExpansion => {
                f.write_str("Expected '(' during preprocessor macro expansion")
            }
            ParseErrorKind::ExpectedEndOfExpression => {
                f.write_str("Expected end of expression during preprocessing")
            }
        }
    }
}
