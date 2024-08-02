use crate::source_files::{Source, SourceFiles};

pub trait Show {
    fn show(&self, w: &mut dyn std::fmt::Write, source_files: &SourceFiles) -> std::fmt::Result;
}

pub fn into_show<T: Show + 'static>(show: T) -> Box<dyn Show> {
    Box::new(show)
}

pub fn error_println(message: &str, source: Source, source_files: &SourceFiles) {
    eprintln!(
        "{}:{}:{}: error: {}",
        source_files.get(source.key).filename(),
        source.location.line,
        source.location.column,
        message,
    )
}
