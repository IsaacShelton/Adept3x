use crate::source_file_cache::SourceFileCache;

pub trait Show {
    fn show(
        &self,
        w: &mut dyn std::fmt::Write,
        source_file_cache: &SourceFileCache,
    ) -> std::fmt::Result;
}
