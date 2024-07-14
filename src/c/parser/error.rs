use crate::{ast::Source, show::Show, source_file_cache::SourceFileCache};
use std::fmt::Display;

#[derive(Clone, Debug)]
pub struct ParseError {
    kind: ParseErrorKind,
    source: Source,
}

impl ParseError {
    pub fn new(kind: ParseErrorKind, source: Source) -> Self {
        Self { kind, source }
    }

    pub fn message(message: &'static str, source: Source) -> Self {
        Self {
            kind: ParseErrorKind::Misc(message),
            source,
        }
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
            self.kind,
        )
    }
}

#[derive(Clone, Debug)]
pub enum ParseErrorKind {
    ExpectedDeclaration,
    InvalidType,
    ExpectedTypeNameOrMemberDeclarationList,
    ExpectedSemicolon,
    ExpectedMemberDeclarator,
    DuplicateEnumMember(String),
    MustBeConstantInteger,
    EnumMemberNameConflictsWithExistingSymbol { name: String },
    UndefinedVariable(String),
    Misc(&'static str),
}

impl ParseErrorKind {
    pub fn at(self, source: Source) -> ParseError {
        ParseError { kind: self, source }
    }
}

impl Display for ParseErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseErrorKind::ExpectedDeclaration => f.write_str("Expected declaration"),
            ParseErrorKind::InvalidType => f.write_str("Invalid type"),
            ParseErrorKind::ExpectedTypeNameOrMemberDeclarationList => {
                f.write_str("Expected type name or member declaration list")
            }
            ParseErrorKind::ExpectedSemicolon => f.write_str("Expected ';'"),
            ParseErrorKind::ExpectedMemberDeclarator => f.write_str("Expected member declarator"),
            ParseErrorKind::DuplicateEnumMember(name) => {
                write!(f, "Duplicate enum member '{name}'")
            }
            ParseErrorKind::MustBeConstantInteger => {
                write!(f, "Must be constant integer expression")
            }
            ParseErrorKind::EnumMemberNameConflictsWithExistingSymbol { name } => {
                write!(
                    f,
                    "Enum member name conflicts with existing symbol '{name}'",
                )
            }
            ParseErrorKind::UndefinedVariable(name) => write!(f, "Undefined variable '{name}'"),
            ParseErrorKind::Misc(message) => f.write_str(message),
        }
    }
}
