use crate::{
    show::Show,
    source_files::{Source, SourceFiles},
    token::{Token, TokenKind},
};
use std::{borrow::Borrow, fmt::Display};

#[derive(Clone, Debug)]
pub struct ParseError {
    pub kind: ParseErrorKind,
    pub source: Source,
}

impl ParseError {
    pub fn expected(
        expected: impl ToString,
        for_reason: Option<impl ToString>,
        got: impl Borrow<Token>,
    ) -> Self {
        match &got.borrow().kind {
            TokenKind::Error(message) => ParseErrorKind::Lexical {
                message: message.into(),
            },
            _ => ParseErrorKind::Expected {
                expected: expected.to_string(),
                for_reason: for_reason.map(|reason| reason.to_string()),
                got: got.borrow().kind.to_string(),
            },
        }
        .at(got.borrow().source)
    }
}

#[derive(Clone, Debug)]
pub enum ParseErrorKind {
    UnexpectedToken {
        unexpected: String,
    },
    Expected {
        expected: String,
        for_reason: Option<String>,
        got: String,
    },
    ExpectedType {
        prefix: Option<String>,
        for_reason: Option<String>,
        got: String,
    },
    UndeclaredType {
        name: String,
    },
    UnrecognizedAnnotation {
        name: String,
    },
    ExpectedTopLevelConstruct,
    UnexpectedAnnotation {
        name: String,
        for_reason: Option<String>,
    },
    ExpectedCommaInTypeParameters,
    ExpectedTypeParameters,
    ExpectedTypeName,
    TypeAliasHasMultipleDefinitions {
        name: String,
    },
    EnumHasMultipleDefinitions {
        name: String,
    },
    DefineHasMultipleDefinitions {
        name: String,
    },
    ExpectedEnumMemberName,
    Lexical {
        message: String,
    },
    CannotCallFunctionsAtGlobalScope,
    IncorrectNumberOfTypeParametersFor {
        name: String,
        got: usize,
        expected: usize,
    },
    ExpectedTypeParameterToBeAType {
        name: String,
        word_for_nth: String,
    },
    GenericsNotSupportedHere,
    NamespaceNotAllowedHere,
    Other {
        message: String,
    },
}

impl ParseErrorKind {
    pub fn at(self, source: Source) -> ParseError {
        ParseError { kind: self, source }
    }
}

impl Show for ParseError {
    fn show(&self, w: &mut dyn std::fmt::Write, source_files: &SourceFiles) -> std::fmt::Result {
        write!(
            w,
            "{}:{}:{}: error: {}",
            source_files.get(self.source.key).filename(),
            self.source.location.line,
            self.source.location.column,
            self.kind
        )
    }
}

impl Display for ParseErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseErrorKind::UnexpectedToken { unexpected } => {
                write!(f, "Unexpected token {}", unexpected)?;
            }
            ParseErrorKind::Expected {
                expected,
                got,
                for_reason,
            } => {
                write!(f, "Expected {}", expected)?;

                if let Some(for_reason) = for_reason {
                    write!(f, " {}", for_reason)?;
                }

                write!(f, ", got {}", got)?;
            }
            ParseErrorKind::ExpectedType {
                prefix,
                for_reason,
                got,
            } => {
                write!(f, "Expected ")?;

                if let Some(prefix) = prefix {
                    write!(f, "{}", prefix)?;
                }

                write!(f, "type")?;

                if let Some(for_reason) = for_reason {
                    write!(f, " {}", for_reason)?;
                }

                write!(f, ", got {}", got)?;
            }
            ParseErrorKind::UndeclaredType { name } => {
                write!(f, "Undeclared type '{}'", name)?;
            }
            ParseErrorKind::UnrecognizedAnnotation { name } => {
                write!(f, "Unrecognized annotation '{}'", name)?;
            }
            ParseErrorKind::ExpectedTopLevelConstruct => {
                write!(f, "Expected top level construct")?;
            }
            ParseErrorKind::UnexpectedAnnotation { name, for_reason } => {
                write!(f, "Unexpected annotation '{}'", name)?;

                if let Some(for_reason) = for_reason {
                    write!(f, " {}", for_reason)?;
                }
            }
            ParseErrorKind::ExpectedCommaInTypeParameters => {
                write!(f, "Expected ',' during type parameters")?;
            }
            ParseErrorKind::ExpectedTypeParameters => {
                write!(f, "Expected type parameters")?;
            }
            ParseErrorKind::ExpectedTypeName => {
                write!(f, "Expected type name")?;
            }
            ParseErrorKind::TypeAliasHasMultipleDefinitions { name } => {
                write!(f, "Type alias '{}' has multiple definitions", name)?;
            }
            ParseErrorKind::EnumHasMultipleDefinitions { name } => {
                write!(f, "Enum '{}' has multiple definitions", name)?;
            }
            ParseErrorKind::DefineHasMultipleDefinitions { name } => {
                write!(f, "Define '{}' has multiple definitions", name)?;
            }
            ParseErrorKind::ExpectedEnumMemberName => {
                write!(f, "Expected enum member name")?;
            }
            ParseErrorKind::CannotCallFunctionsAtGlobalScope => {
                write!(f, "Cannot call functions at global scope")?;
            }
            ParseErrorKind::IncorrectNumberOfTypeParametersFor {
                name,
                got,
                expected,
            } => {
                write!(
                    f,
                    "Incorrect number of type parameters for '{}' (got {}, expected {})",
                    name, got, expected
                )?;
            }
            ParseErrorKind::ExpectedTypeParameterToBeAType { name, word_for_nth } => {
                write!(
                    f,
                    "Expected {} type parameter to '{}' to be a type",
                    word_for_nth, name
                )?;
            }
            ParseErrorKind::GenericsNotSupportedHere => {
                write!(f, "Generics not supported here")?;
            }
            ParseErrorKind::NamespaceNotAllowedHere => {
                write!(f, "Namespace not allowed here")?;
            }
            ParseErrorKind::Other { message } | ParseErrorKind::Lexical { message } => {
                write!(f, "{}", message)?;
            }
        }

        Ok(())
    }
}
