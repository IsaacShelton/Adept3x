use crate::{
    show::Show,
    source_files::{Source, SourceFiles},
};
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
    fn show(&self, w: &mut dyn std::fmt::Write, source_files: &SourceFiles) -> std::fmt::Result {
        write!(
            w,
            "{}:{}:{}: error: {}",
            source_files.get(self.source.key).filename(),
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
    UndeclaredVariable(String),
    UndeclaredType(String),
    CannotContainNulInNullTerminatedString,
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
            ParseErrorKind::UndeclaredVariable(name) => write!(f, "Undeclared variable '{name}'"),
            ParseErrorKind::UndeclaredType(name) => write!(f, "Undeclared type '{name}'"),
            ParseErrorKind::CannotContainNulInNullTerminatedString => {
                write!(f, "Cannot contain NUL byte in C-String'")
            }
            ParseErrorKind::Misc(message) => f.write_str(message),
        }
    }
}
