use crate::{Show, minimal_filename};
use colored::Colorize;
use source_files::{Source, SourceFiles};
use std::{cmp::Ordering, path::Path};

#[derive(Debug)]
pub struct ErrorDiagnostic {
    message: String,
    source: Option<Source>,
    postfix: Option<&'static str>,
}

impl ErrorDiagnostic {
    pub fn ice(message: impl ToString, source: Option<Source>) -> Self {
        Self {
            message: format!("internal compiler error => {}", message.to_string()),
            source: source,
            postfix: None,
        }
    }

    pub fn new(message: impl ToString, source: Source) -> Self {
        Self {
            message: message.to_string(),
            source: Some(source),
            postfix: None,
        }
    }

    pub fn new_maybe_source(message: impl ToString, source: Option<Source>) -> Self {
        Self {
            message: message.to_string(),
            source,
            postfix: None,
        }
    }

    pub fn plain(message: impl ToString) -> Self {
        Self {
            message: message.to_string(),
            source: None,
            postfix: None,
        }
    }

    pub fn with_postfix(mut self, postfix: Option<&'static str>) -> Self {
        self.postfix = postfix;
        self
    }

    pub fn cmp_with(&self, other: &Self, source_files: &SourceFiles) -> Ordering {
        if self.source.is_none() && other.source.is_none() {
            return Ordering::Equal;
        }

        if self.source.is_none() && other.source.is_some() {
            return Ordering::Less;
        }

        if self.source.is_some() && other.source.is_none() {
            return Ordering::Greater;
        }

        let a = self.source.unwrap();
        let b = other.source.unwrap();

        let filename_ordering = source_files
            .get(a.key)
            .filename()
            .cmp(source_files.get(b.key).filename());

        filename_ordering
            .then_with(|| a.location.cmp(&b.location))
            .then_with(|| self.message.cmp(&other.message))
            .then_with(|| self.postfix.cmp(&other.postfix))
    }
}

impl Show for ErrorDiagnostic {
    fn show(
        &self,
        w: &mut dyn std::fmt::Write,
        source_files: &SourceFiles,
        project_root: Option<&Path>,
    ) -> std::fmt::Result {
        if let Some(source) = self.source {
            write!(
                w,
                "{}:{}:{}: {} {}{}",
                minimal_filename(source, source_files, project_root),
                source.location.line,
                source.location.column,
                "error:".red().bold(),
                self.message,
                self.postfix.unwrap_or(""),
            )
        } else {
            write!(
                w,
                "{} {}{}",
                "error:".red().bold(),
                self.message,
                self.postfix.unwrap_or("").red().bold(),
            )
        }
    }
}
