use colored::Colorize;
use std::{error::Error, fmt::Display};

#[derive(Copy, Clone, Debug)]
pub enum CompilerErrorKind {
    CommandLine,
    Lex,
    Parse,
    Lower,
    Backend,
}

impl Into<&str> for CompilerErrorKind {
    fn into(self) -> &'static str {
        match self {
            Self::CommandLine => "cli error",
            Self::Lex => "lex error",
            Self::Parse => "syntax error",
            Self::Lower => "lower error",
            Self::Backend => "translation error",
        }
    }
}

#[derive(Debug)]
pub struct CompilerError {
    message: String,
    kind: CompilerErrorKind,
}

impl CompilerError {
    pub fn during_lower(message: impl ToString) -> Self {
        Self {
            message: message.to_string(),
            kind: CompilerErrorKind::Lower,
        }
    }

    pub fn during_backend(message: impl ToString) -> Self {
        Self {
            message: message.to_string(),
            kind: CompilerErrorKind::Backend,
        }
    }
}

impl Display for CompilerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}{}{}",
            Into::<&str>::into(self.kind).red(),
            ": ".red(),
            self.message
        )?;

        Ok(())
    }
}

impl Error for CompilerError {}
